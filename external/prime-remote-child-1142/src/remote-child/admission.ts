/**
 * Parent-side admission host for prime-agent.remote-child/v1.
 *
 * Owns invitations, durable admission, leases, fencing, parent commands, and
 * the remote-child state machine. It is transport-agnostic: frames arrive
 * through {@link dispatchWorkerFrame} and host responses leave through the
 * injected {@link RemoteChildTransport}. Sender identity is always derived
 * from the authenticated lease + ledger, never from a payload field.
 */
import { randomBytes, randomUUID } from "node:crypto";
import { mkdirSync } from "node:fs";
import { join } from "node:path";
import { generateOpaqueToken, publicKeyHash, sha256Hex, verifyPossession } from "./crypto.js";
import { commandKeyFor, RemoteChildJournal } from "./journal.js";
import type { RemoteChildTransport } from "./transport.js";
import type {
	RemoteChildAdmissionCapsule,
	RemoteChildAdmissionOutcome,
	RemoteChildAttachOutcome,
	RemoteChildCommandReceipt,
	RemoteChildInvitationRecord,
	RemoteChildInvitationSpec,
	RemoteChildLease,
	RemoteChildMutationHeader,
	RemoteChildParentCommand,
	RemoteChildRecord,
	RemoteChildRegistryEntry,
	RemoteChildState,
	RemoteChildWorkerFrame,
	RemoteChildWorkerMessage,
	RemoteChildWorkloadClaims,
} from "./types.js";
import {
	REMOTE_CHILD_ACTIVE_STATES,
	REMOTE_CHILD_DEFAULT_DISCONNECT_GRACE_MS,
	REMOTE_CHILD_DEFAULT_INVITATION_TTL_MS,
	REMOTE_CHILD_DEFAULT_LEASE_TTL_MS,
	REMOTE_CHILD_DEFAULT_STOP_DEADLINE_MS,
	REMOTE_CHILD_MAX_ATTEMPTS,
	REMOTE_CHILD_MAX_CHILDREN_PER_PARENT,
	REMOTE_CHILD_MAX_MESSAGE_CHARS,
	REMOTE_CHILD_MAX_PENDING_PARENT_COMMANDS,
	REMOTE_CHILD_PROTOCOL,
	REMOTE_CHILD_TERMINAL_STATES,
} from "./types.js";

export interface RemoteChildAdmissionHostOptions {
	storeDir: string;
	parentSessionId: string;
	parentSessionFile: string;
	/** Depth of the parent session; the remote child is parentDepth + 1. */
	depth: number;
	transport: RemoteChildTransport;
	clock?: () => number;
	/** Deliver a worker→parent message; the host has already derived the sender. */
	onWorkerMessage?: (message: RemoteChildWorkerMessage) => void | Promise<void>;
	/** Audit hook fired when a child reaches a terminal state. */
	onTerminal?: (entry: RemoteChildRegistryEntry) => void;
	invitationTtlMs?: number;
	leaseTtlMs?: number;
	stopDeadlineMs?: number;
	disconnectGraceMs?: number;
	maxChildren?: number;
	maxPendingCommands?: number;
	maxAttempts?: number;
}

export interface RemoteChildInviteOutcome {
	invitationId: string;
	expiresAt: number;
}

interface HostLimits {
	invitationTtlMs: number;
	leaseTtlMs: number;
	stopDeadlineMs: number;
	disconnectGraceMs: number;
	maxChildren: number;
	maxPendingCommands: number;
	maxAttempts: number;
}

function workloadClaimsEqual(left: RemoteChildWorkloadClaims, right: RemoteChildWorkloadClaims): boolean {
	const keys = new Set([...Object.keys(left), ...Object.keys(right)]);
	for (const key of keys) {
		const l = (left as Record<string, unknown>)[key];
		const r = (right as Record<string, unknown>)[key];
		if (l !== r) return false;
	}
	return true;
}

export class RemoteChildAdmissionHost {
	private readonly journal: RemoteChildJournal;
	private readonly invitations = new Map<string, RemoteChildInvitationRecord>();
	private readonly children = new Map<string, RemoteChildRecord>();
	private readonly commands = new Map<string, RemoteChildParentCommand>();
	private readonly deleted = new Set<string>();
	private readonly leases = new Map<string, RemoteChildLease>();
	private readonly leaseByChild = new Map<string, RemoteChildLease>();
	private readonly connected = new Set<string>();
	private readonly seenNonces = new Set<string>();
	private readonly idempotency = new Map<string, Map<string, unknown>>();
	private readonly admissionIndex = new Map<string, string>();
	private readonly limits: HostLimits;

	constructor(private readonly options: RemoteChildAdmissionHostOptions) {
		this.limits = {
			invitationTtlMs: options.invitationTtlMs ?? REMOTE_CHILD_DEFAULT_INVITATION_TTL_MS,
			leaseTtlMs: options.leaseTtlMs ?? REMOTE_CHILD_DEFAULT_LEASE_TTL_MS,
			stopDeadlineMs: options.stopDeadlineMs ?? REMOTE_CHILD_DEFAULT_STOP_DEADLINE_MS,
			disconnectGraceMs: options.disconnectGraceMs ?? REMOTE_CHILD_DEFAULT_DISCONNECT_GRACE_MS,
			maxChildren: options.maxChildren ?? REMOTE_CHILD_MAX_CHILDREN_PER_PARENT,
			maxPendingCommands: options.maxPendingCommands ?? REMOTE_CHILD_MAX_PENDING_PARENT_COMMANDS,
			maxAttempts: options.maxAttempts ?? REMOTE_CHILD_MAX_ATTEMPTS,
		};
		mkdirSync(options.storeDir, { recursive: true, mode: 0o700 });
		const dir = join(options.storeDir, this.parentDirName());
		mkdirSync(dir, { recursive: true, mode: 0o700 });
		this.journal = RemoteChildJournal.open(join(dir, "journal.jsonl"));
		const replay = this.journal.replay();
		if (replay.invitations.size === 0 && replay.children.size === 0 && replay.commands.size === 0) {
			this.journal.append({ v: 1, op: "meta", at: this.now(), sessionsDir: this.options.parentSessionFile });
		}
		for (const [hash, invitation] of replay.invitations) this.invitations.set(hash, invitation);
		for (const [childId, child] of replay.children) {
			this.children.set(childId, child);
			this.admissionIndex.set(child.admissionRequestId, childId);
		}
		for (const [key, command] of replay.commands) this.commands.set(key, command);
		for (const childId of replay.deleted) this.deleted.add(childId);
	}

	get parentSessionId(): string {
		return this.options.parentSessionId;
	}

	private parentDirName(): string {
		return `parent-${sha256Hex(this.options.parentSessionId).slice(0, 24)}`;
	}

	private now(): number {
		return (this.options.clock ?? Date.now)();
	}

	private allocateChildId(): string {
		return `remote-${randomBytes(6).toString("hex")}`;
	}

	private defaultChildName(childId: string): string {
		return `remote-worker-${childId.slice(-8)}`;
	}

	close(): void {
		this.journal.close();
	}

	/** Create a single-use, short-TTL invitation bound to this parent + workload. */
	invite(spec: RemoteChildInvitationSpec): RemoteChildInviteOutcome {
		if (spec.parentSessionId !== this.options.parentSessionId) {
			throw new Error("invitation must target this host's parent session");
		}
		const active = this.activeChildren().length;
		if (active >= this.limits.maxChildren) {
			throw new Error(`parent session already has ${active} active remote children`);
		}
		const token = generateOpaqueToken();
		const now = this.now();
		const ttlMs = spec.ttlMs ?? this.limits.invitationTtlMs;
		const record: RemoteChildInvitationRecord = {
			invitationHash: token.hash,
			state: "open",
			issuedAt: now,
			expiresAt: now + ttlMs,
			parentSessionId: spec.parentSessionId,
			parentSessionFile: spec.parentSessionFile,
			depth: spec.depth,
			name: spec.name,
			workload: spec.workload,
		};
		this.journal.append({ v: 1, op: "invitation", at: now, record });
		this.invitations.set(token.hash, record);
		return { invitationId: token.plaintext, expiresAt: record.expiresAt };
	}

	revokeInvitation(invitationId: string): boolean {
		const hash = sha256Hex(invitationId);
		const invitation = this.invitations.get(hash);
		if (!invitation || invitation.state !== "open") return false;
		const updated: RemoteChildInvitationRecord = { ...invitation, state: "revoked" };
		this.journal.append({ v: 1, op: "invitation", at: this.now(), record: updated });
		this.invitations.set(hash, updated);
		return true;
	}

	/** Entry point for frames arriving from a worker over the transport. */
	async dispatchWorkerFrame(childId: string, frame: RemoteChildWorkerFrame): Promise<void> {
		switch (frame.type) {
			case "admit": {
				const outcome = await this.processAdmit(frame);
				if (outcome.status === "admitted") {
					this.transport().sendToWorker(childId, {
						type: "admitted",
						capsule: outcome.capsule,
						admissionRequestId: frame.admissionRequestId,
					});
				} else {
					this.transport().sendToWorker(childId, {
						type: "admission_denied",
						reason: outcome.reason,
						admissionRequestId: frame.admissionRequestId,
					});
				}
				return;
			}
			case "attach": {
				const outcome = this.processAttach(frame);
				if (outcome.status === "attached") {
					this.transport().sendToWorker(childId, { type: "lease_renewed", lease: outcome.lease });
				} else {
					this.transport().sendToWorker(childId, { type: "lease_expired", reason: outcome.reason });
				}
				return;
			}
			case "start_ack":
			case "heartbeat":
			case "checkpoint":
			case "message":
			case "complete":
			case "fail":
			case "command_ack":
				await this.processMutation(childId, frame);
				return;
		}
	}

	private transport(): RemoteChildTransport {
		return this.options.transport;
	}

	async processAdmit(frame: Extract<RemoteChildWorkerFrame, { type: "admit" }>): Promise<RemoteChildAdmissionOutcome> {
		const invitationHash = sha256Hex(frame.invitationId);

		// Idempotent retry of an already-committed admission: identical claims
		// and invitation return the same committed child, never a second one.
		const existingChildId = this.admissionIndex.get(frame.admissionRequestId);
		if (existingChildId) {
			const existing = this.children.get(existingChildId);
			if (!existing) return { status: "denied", reason: "admission reference is inconsistent" };
			if (
				existing.admissionInvitationHash !== invitationHash ||
				!workloadClaimsEqual(existing.workload, frame.workloadClaims)
			) {
				return { status: "denied", reason: "admission request id was already used with different claims" };
			}
			return { status: "admitted", capsule: this.mintCapsule(existing) };
		}

		const invitation = this.invitations.get(invitationHash);
		if (!invitation) return { status: "denied", reason: "unknown invitation" };
		if (invitation.state !== "open") return { status: "denied", reason: "invitation is not open" };
		if (this.now() >= invitation.expiresAt) return { status: "denied", reason: "invitation expired" };
		if (!workloadClaimsEqual(invitation.workload, frame.workloadClaims)) {
			return { status: "denied", reason: "workload claims mismatch" };
		}
		if (!frame.supportedVersions.includes(REMOTE_CHILD_PROTOCOL)) {
			return { status: "denied", reason: "unsupported protocol version" };
		}
		if (!verifyPossession(frame.workerPublicKey, frame.nonce, frame.possessionSignature)) {
			return { status: "denied", reason: "worker possession proof failed" };
		}
		if (this.seenNonces.has(frame.nonce)) return { status: "denied", reason: "nonce replay" };
		this.seenNonces.add(frame.nonce);

		const now = this.now();
		const childId = this.allocateChildId();
		const sessionId = randomUUID();
		const child: RemoteChildRecord = {
			protocolVersion: REMOTE_CHILD_PROTOCOL,
			childId,
			sessionId,
			parentSessionId: this.options.parentSessionId,
			parentSessionFile: this.options.parentSessionFile,
			depth: this.options.depth + 1,
			name: invitation.name ?? this.defaultChildName(childId),
			workload: frame.workloadClaims,
			state: "admitted",
			attempt: 1,
			connectionEpoch: 1,
			workerPublicKeyHash: publicKeyHash(frame.workerPublicKey),
			issuedAt: now,
			admittedAt: now,
			lastEventSeq: 0,
			admissionRequestId: frame.admissionRequestId,
			admissionInvitationHash: invitationHash,
			parentCommandHighWater: 0,
			acknowledgedCommandSeq: 0,
		};

		// Durable commit: consume the invitation and allocate the child, both
		// fsynced before any start permit exists.
		this.journal.append({
			v: 1,
			op: "invitation",
			at: now,
			record: { ...invitation, state: "consumed", consumedBy: frame.admissionRequestId, consumedAt: now },
		});
		this.journal.append({ v: 1, op: "child", at: now, record: child });

		// Only once recovery can reproduce the committed child do we mint the
		// lease and return ADMITTED.
		const replay = this.journal.replay();
		if (!replay.children.has(childId)) {
			throw new Error("admission commit did not reproduce the child; refusing to issue a start permit");
		}

		this.invitations.set(invitationHash, {
			...invitation,
			state: "consumed",
			consumedBy: frame.admissionRequestId,
			consumedAt: now,
		});
		this.children.set(childId, child);
		this.admissionIndex.set(frame.admissionRequestId, childId);

		return { status: "admitted", capsule: this.mintCapsule(child) };
	}

	processAttach(frame: Extract<RemoteChildWorkerFrame, { type: "attach" }>): RemoteChildAttachOutcome {
		const lease = this.leases.get(frame.leaseId);
		if (!lease || this.now() >= lease.expiresAt) return { status: "denied", reason: "lease expired" };
		const child = this.children.get(lease.childId);
		if (!child || this.deleted.has(child.childId)) return { status: "denied", reason: "child not found" };
		if (REMOTE_CHILD_TERMINAL_STATES.has(child.state)) return { status: "denied", reason: "child is terminal" };
		if (frame.attempt !== child.attempt || frame.connectionEpoch !== child.connectionEpoch) {
			return { status: "denied", reason: "attempt or epoch mismatch" };
		}
		if (child.workerPublicKeyHash !== publicKeyHash(frame.workerPublicKey)) {
			return { status: "denied", reason: "worker key mismatch" };
		}
		if (!verifyPossession(frame.workerPublicKey, frame.leaseId, frame.possessionSignature)) {
			return { status: "denied", reason: "worker possession proof failed" };
		}

		// Re-attach fences the previous epoch by advancing it.
		child.connectionEpoch += 1;
		if (child.state === "disconnected") child.state = "running";
		this.journal.append({ v: 1, op: "child", at: this.now(), record: child });
		this.children.set(child.childId, child);
		this.connected.add(child.childId);
		this.redeliverPendingCommands(child);
		const renewed = this.mintLease(child);
		return { status: "attached", lease: renewed };
	}

	private rejectMutation(
		childId: string,
		frame: RemoteChildWorkerFrame & RemoteChildMutationHeader,
		reason: string,
	): void {
		if (frame.type === "message") {
			this.transport().sendToWorker(childId, { type: "message_receipt", status: "rejected", reason });
			return;
		}
		if (frame.type === "checkpoint") {
			this.transport().sendToWorker(childId, { type: "checkpoint_receipt", status: "rejected", reason });
			return;
		}
		this.transport().sendToWorker(childId, { type: "error", reason });
	}

	private async processMutation(
		childId: string,
		frame: RemoteChildWorkerFrame & RemoteChildMutationHeader,
	): Promise<void> {
		const child = this.children.get(childId);
		if (!child || this.deleted.has(childId)) {
			this.rejectMutation(childId, frame, "unknown child");
			return;
		}
		if (REMOTE_CHILD_TERMINAL_STATES.has(child.state) && frame.type !== "command_ack") {
			this.transport().sendToWorker(childId, {
				type: "terminal_notice",
				state: child.state,
				reason: child.terminalReason,
			});
			return;
		}
		if (!this.connected.has(childId)) {
			this.rejectMutation(childId, frame, "not attached");
			return;
		}
		if (frame.attempt !== child.attempt || frame.connectionEpoch !== child.connectionEpoch) {
			this.rejectMutation(childId, frame, "attempt or epoch mismatch");
			return;
		}
		if (!Number.isInteger(frame.sequence) || frame.sequence < 1) {
			this.rejectMutation(childId, frame, "invalid sequence");
			return;
		}

		const recorded = this.idempotency.get(childId)?.get(frame.idempotencyKey);
		if (recorded !== undefined) {
			this.replyIdempotent(childId, frame, recorded);
			return;
		}
		if (frame.sequence <= child.lastEventSeq) {
			this.rejectMutation(childId, frame, "out-of-order sequence");
			return;
		}

		child.lastEventSeq = frame.sequence;
		await this.applyMutation(childId, child, frame);
	}

	private replyIdempotent(
		childId: string,
		frame: RemoteChildWorkerFrame & RemoteChildMutationHeader,
		recorded: unknown,
	): void {
		switch (frame.type) {
			case "message":
				this.transport().sendToWorker(childId, {
					type: "message_receipt",
					status: "duplicate",
					messageId: recorded as string,
				});
				return;
			case "checkpoint":
				this.transport().sendToWorker(childId, { type: "checkpoint_receipt", status: "accepted" });
				return;
			default:
				return;
		}
	}

	private async applyMutation(
		childId: string,
		child: RemoteChildRecord,
		frame: RemoteChildWorkerFrame & RemoteChildMutationHeader,
	): Promise<void> {
		switch (frame.type) {
			case "start_ack": {
				if (child.state === "admitted") child.state = "running";
				child.startedAt = child.startedAt ?? this.now();
				this.persistChild(child);
				return;
			}
			case "heartbeat": {
				child.lastHeartbeatAt = this.now();
				this.children.set(childId, child);
				const lease = this.mintLease(child);
				this.transport().sendToWorker(childId, { type: "lease_renewed", lease });
				return;
			}
			case "checkpoint": {
				child.checkpointRef = frame.checkpointRef;
				child.checkpointHash = frame.checkpointHash;
				child.checkpointSeq = frame.checkpointSeq;
				this.persistChild(child);
				this.recordIdempotency(childId, frame.idempotencyKey, {});
				this.transport().sendToWorker(childId, { type: "checkpoint_receipt", status: "accepted" });
				return;
			}
			case "message": {
				if (typeof frame.text !== "string" || frame.text.trim().length === 0) {
					this.transport().sendToWorker(childId, {
						type: "message_receipt",
						status: "rejected",
						reason: "empty message",
					});
					return;
				}
				if (frame.text.length > REMOTE_CHILD_MAX_MESSAGE_CHARS) {
					this.transport().sendToWorker(childId, {
						type: "message_receipt",
						status: "rejected",
						reason: "message too long",
					});
					return;
				}
				const messageId = `agentmsg_${randomUUID()}`;
				// Durable high-water mark first so a restart cannot redeliver.
				this.persistChild(child);
				this.recordIdempotency(childId, frame.idempotencyKey, messageId);
				const message: RemoteChildWorkerMessage = {
					childId,
					sessionId: child.sessionId,
					parentSessionId: child.parentSessionId,
					messageId,
					text: frame.text,
					attempt: child.attempt,
					connectionEpoch: child.connectionEpoch,
					sequence: frame.sequence,
				};
				await this.options.onWorkerMessage?.(message);
				this.transport().sendToWorker(childId, { type: "message_receipt", status: "delivered", messageId });
				return;
			}
			case "complete": {
				this.reachTerminal(child, "completed", undefined);
				return;
			}
			case "fail": {
				const reason = sanitizeErrorSummary(frame.reason);
				this.reachTerminal(child, "failed", reason);
				return;
			}
			case "command_ack": {
				const command = this.commands.get(commandKeyFor(childId, frame.commandSeq));
				if (!command) {
					this.transport().sendToWorker(childId, { type: "error", reason: "unknown command" });
					return;
				}
				child.acknowledgedCommandSeq = Math.max(child.acknowledgedCommandSeq, frame.commandSeq);
				if (command.kind === "stop" && child.state === "cancelling") {
					if (frame.acknowledged) {
						this.reachTerminal(child, "cancelled", command.reason ?? "stopped by parent");
						return;
					}
					this.reachTerminal(child, "failed", command.reason ?? "worker rejected stop");
					return;
				}
				this.persistChild(child);
				this.recordIdempotency(childId, frame.idempotencyKey, {});
				return;
			}
		}
	}

	private recordIdempotency(childId: string, idempotencyKey: string, result: unknown): void {
		let map = this.idempotency.get(childId);
		if (!map) {
			map = new Map();
			this.idempotency.set(childId, map);
		}
		map.set(idempotencyKey, result);
	}

	private persistChild(child: RemoteChildRecord): void {
		this.journal.append({ v: 1, op: "child", at: this.now(), record: child });
		this.children.set(child.childId, child);
	}

	private reachTerminal(child: RemoteChildRecord, state: RemoteChildState, reason: string | undefined): void {
		child.state = state;
		child.terminalAt = this.now();
		child.terminalReason = reason;
		this.journal.append({ v: 1, op: "child", at: child.terminalAt, record: child });
		this.children.set(child.childId, child);
		this.connected.delete(child.childId);
		this.revokeLease(child.childId);
		this.transport().sendToWorker(child.childId, { type: "terminal_notice", state, reason });
		this.options.onTerminal?.(this.toRegistryEntry(child));
	}

	private mintCapsule(child: RemoteChildRecord): RemoteChildAdmissionCapsule {
		return {
			protocolVersion: REMOTE_CHILD_PROTOCOL,
			childId: child.childId,
			sessionId: child.sessionId,
			parentSessionId: child.parentSessionId,
			depth: child.depth,
			attempt: child.attempt,
			connectionEpoch: child.connectionEpoch,
			lease: this.mintLease(child),
		};
	}

	private mintLease(child: RemoteChildRecord): RemoteChildLease {
		const previous = this.leaseByChild.get(child.childId);
		if (previous) this.leases.delete(previous.leaseId);
		const lease: RemoteChildLease = {
			leaseId: generateOpaqueToken().plaintext,
			childId: child.childId,
			sessionId: child.sessionId,
			parentSessionId: child.parentSessionId,
			attempt: child.attempt,
			connectionEpoch: child.connectionEpoch,
			expiresAt: this.now() + this.limits.leaseTtlMs,
			workerPublicKeyHash: child.workerPublicKeyHash,
		};
		this.leases.set(lease.leaseId, lease);
		this.leaseByChild.set(child.childId, lease);
		return lease;
	}

	private revokeLease(childId: string): void {
		const lease = this.leaseByChild.get(childId);
		if (lease) this.leases.delete(lease.leaseId);
		this.leaseByChild.delete(childId);
	}

	private redeliverPendingCommands(child: RemoteChildRecord): void {
		for (const command of this.commands.values()) {
			if (command.childIdForRouting !== child.childId) continue;
			if (command.commandSeq <= child.acknowledgedCommandSeq) continue;
			this.transport().sendToWorker(child.childId, { type: "parent_command", command });
		}
	}

	/** Issue a parent→worker command (steer / follow-up / stop). */
	issueCommand(
		childId: string,
		kind: "steer" | "follow_up" | "stop",
		text: string,
		reason?: string,
	): RemoteChildCommandReceipt {
		const child = this.children.get(childId);
		if (!child || this.deleted.has(childId)) return { status: "rejected", commandSeq: 0, reason: "unknown child" };
		if (REMOTE_CHILD_TERMINAL_STATES.has(child.state))
			return { status: "rejected", commandSeq: 0, reason: "child is terminal" };
		const pending = this.pendingCommandCount(childId);
		if (pending >= this.limits.maxPendingCommands) {
			return { status: "rejected", commandSeq: 0, reason: "too many pending commands" };
		}

		child.parentCommandHighWater += 1;
		const commandSeq = child.parentCommandHighWater;
		const command: RemoteChildParentCommand = {
			commandId: `remote-cmd-${randomUUID()}`,
			commandSeq,
			kind,
			text,
			reason,
			issuedAt: this.now(),
			deadlineAt: kind === "stop" ? this.now() + this.limits.stopDeadlineMs : undefined,
			childIdForRouting: childId,
		};
		this.journal.append({ v: 1, op: "command", at: this.now(), childId, command });
		this.commands.set(commandKeyFor(childId, commandSeq), command);

		if (kind === "stop") {
			child.state = "cancelling";
		}
		this.persistChild(child);

		this.transport().sendToWorker(childId, { type: "parent_command", command });
		return { status: "queued", commandSeq };
	}

	steer(childId: string, text: string): RemoteChildCommandReceipt {
		return this.issueCommand(childId, "steer", text);
	}

	followUp(childId: string, text: string): RemoteChildCommandReceipt {
		return this.issueCommand(childId, "follow_up", text);
	}

	stop(childId: string, reason = "stopped by parent"): RemoteChildCommandReceipt {
		return this.issueCommand(childId, "stop", "", reason);
	}

	delete(childId: string): boolean {
		const child = this.children.get(childId);
		if (!child) return false;
		if (this.deleted.has(childId)) return true;
		this.revokeLease(childId);
		this.connected.delete(childId);
		child.connectionEpoch += 1; // fence late frames
		this.journal.append({ v: 1, op: "child", at: this.now(), record: child });
		this.journal.append({ v: 1, op: "delete", at: this.now(), childId, reason: "user" });
		this.deleted.add(childId);
		return true;
	}

	/** Parent closure: revoke open invitations and issue stop to every live child. */
	closeChildren(reason: string): void {
		for (const [hash, invitation] of this.invitations) {
			if (invitation.state !== "open") continue;
			this.invitations.set(hash, { ...invitation, state: "revoked" });
			this.journal.append({ v: 1, op: "invitation", at: this.now(), record: { ...invitation, state: "revoked" } });
		}
		for (const child of [...this.children.values()]) {
			if (this.deleted.has(child.childId) || REMOTE_CHILD_TERMINAL_STATES.has(child.state)) continue;
			this.issueCommand(child.childId, "stop", "", reason);
		}
	}

	/**
	 * Advance time and collect expiry transitions. Never fabricates a
	 * `cancelled`: a stop without an ACK can only become `uncertain` or
	 * `lease_expired`.
	 */
	collectExpiry(): RemoteChildRegistryEntry[] {
		const now = this.now();
		const transitions: RemoteChildRegistryEntry[] = [];
		for (const child of [...this.children.values()]) {
			if (this.deleted.has(child.childId) || REMOTE_CHILD_TERMINAL_STATES.has(child.state)) continue;
			if (child.state === "cancelling") {
				const stopDeadline = this.stopDeadlineFor(child);
				if (stopDeadline !== undefined && now >= stopDeadline) {
					const lease = this.leaseByChild.get(child.childId);
					const expiredLease = lease === undefined || now >= lease.expiresAt;
					this.reachTerminal(
						child,
						expiredLease ? "lease_expired" : "uncertain",
						"stop was not acknowledged in time",
					);
					transitions.push(this.toRegistryEntry(child));
				}
				continue;
			}
			const lease = this.leaseByChild.get(child.childId);
			if (!lease) continue;
			if (now >= lease.expiresAt) {
				if (
					child.state === "disconnected" &&
					child.disconnectDeadlineAt !== undefined &&
					now >= child.disconnectDeadlineAt
				) {
					this.reachTerminal(child, "lease_expired", "lease expired and was not renewed in time");
					transitions.push(this.toRegistryEntry(child));
				} else if (REMOTE_CHILD_ACTIVE_STATES.has(child.state)) {
					child.state = "disconnected";
					child.disconnectDeadlineAt = now + this.limits.disconnectGraceMs;
					this.persistChild(child);
					this.connected.delete(child.childId);
					transitions.push(this.toRegistryEntry(child));
				}
			}
		}
		return transitions;
	}

	private stopDeadlineFor(child: RemoteChildRecord): number | undefined {
		for (const command of this.commands.values()) {
			if (
				command.childIdForRouting === child.childId &&
				command.kind === "stop" &&
				command.commandSeq > child.acknowledgedCommandSeq
			) {
				return command.deadlineAt;
			}
		}
		return undefined;
	}

	private pendingCommandCount(childId: string): number {
		const child = this.children.get(childId);
		if (!child) return 0;
		let count = 0;
		for (const command of this.commands.values()) {
			if (command.childIdForRouting === childId && command.commandSeq > child.acknowledgedCommandSeq) count += 1;
		}
		return count;
	}

	private activeChildren(): RemoteChildRecord[] {
		return [...this.children.values()].filter(
			(child) => !this.deleted.has(child.childId) && !REMOTE_CHILD_TERMINAL_STATES.has(child.state),
		);
	}

	listEntries(parentSessionId?: string): RemoteChildRegistryEntry[] {
		const filter = parentSessionId ?? this.options.parentSessionId;
		return [...this.children.values()]
			.filter((child) => child.parentSessionId === filter && !this.deleted.has(child.childId))
			.map((child) => this.toRegistryEntry(child));
	}

	private toRegistryEntry(child: RemoteChildRecord): RemoteChildRegistryEntry {
		return {
			rlm_child_id: child.childId,
			session_id: child.sessionId,
			session_name: child.name,
			session_dir: "",
			status:
				child.state === "completed"
					? "completed"
					: REMOTE_CHILD_TERMINAL_STATES.has(child.state)
						? "error"
						: "running",
			runtime: "remote",
			remote_state: child.state,
			attempt: child.attempt,
			connection_epoch: child.connectionEpoch,
			last_heartbeat_at: child.lastHeartbeatAt ?? null,
			parent_session_id: child.parentSessionId,
		};
	}

	/** Internal read access for tests and the registry provider. */
	debugState(): {
		children: RemoteChildRecord[];
		invitations: RemoteChildInvitationRecord[];
		commands: RemoteChildParentCommand[];
	} {
		return {
			children: [...this.children.values()],
			invitations: [...this.invitations.values()],
			commands: [...this.commands.values()],
		};
	}
}

function sanitizeErrorSummary(reason: string): string {
	const trimmed = reason.trim().slice(0, 500);
	return trimmed || "worker reported a terminal failure";
}
