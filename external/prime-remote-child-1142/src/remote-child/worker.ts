/**
 * Mock remote worker for the Core MVP conformance suite.
 *
 * Models a GitHub-Actions-side worker that must receive a durable ADMITTED
 * before it may call a model or touch a checkout. It owns an Ed25519 keypair
 * (injectable so a "restart" reuses the same identity), drives the transport,
 * and records everything the host sends it for test assertions.
 */
import { randomUUID } from "node:crypto";
import { type Ed25519Keypair, generateCapabilityKeypair, signPossession } from "./crypto.js";
import type { RemoteChildTransport } from "./transport.js";
import type {
	RemoteChildAdmissionCapsule,
	RemoteChildHostFrame,
	RemoteChildLease,
	RemoteChildParentCommand,
	RemoteChildState,
	RemoteChildWorkloadClaims,
} from "./types.js";
import { REMOTE_CHILD_PROTOCOL } from "./types.js";

export interface MockRemoteWorkerAdmitParams {
	invitationId: string;
	admissionRequestId: string;
	workloadClaims: RemoteChildWorkloadClaims;
	supportedVersions?: string[];
}

export interface MockRemoteWorkerOptions {
	transport: RemoteChildTransport;
	keypair?: Ed25519Keypair;
	/** Auto-ack parent commands (steer/follow-up/stop) on receipt. */
	autoAckCommands?: boolean;
	onCommand?: (command: RemoteChildParentCommand) => void;
}

interface PendingResolver<T> {
	resolve: (value: T) => void;
	reject: (error: Error) => void;
}

export class RemoteChildDeniedError extends Error {}

export class MockRemoteWorker {
	private readonly transport: RemoteChildTransport;
	private readonly keypair: Ed25519Keypair;
	private readonly autoAckCommands: boolean;
	private readonly onCommand?: (command: RemoteChildParentCommand) => void;
	private seq = 0;
	private state: "idle" | "admitting" | "admitted" | "attached" | "running" | "terminal" = "idle";
	private _capsule?: RemoteChildAdmissionCapsule;
	private _lease?: RemoteChildLease;
	private _startedModel = false;
	private _terminalState?: RemoteChildState;
	private readonly receivedCommands: RemoteChildParentCommand[] = [];
	private readonly pendingAdmissions = new Map<string, PendingResolver<RemoteChildAdmissionCapsule>>();
	private readonly pendingAttaches = new Map<string, PendingResolver<RemoteChildLease>>();
	private readonly pendingMessages = new Map<string, PendingResolver<{ status: string; messageId?: string }>>();
	private readonly pendingCheckpoints = new Map<string, PendingResolver<void>>();
	private readonly pendingHeartbeats = new Map<string, PendingResolver<void>>();

	constructor(options: MockRemoteWorkerOptions) {
		this.transport = options.transport;
		this.keypair = options.keypair ?? generateCapabilityKeypair();
		this.autoAckCommands = options.autoAckCommands ?? true;
		this.onCommand = options.onCommand;
		options.transport.onHostFrame((_childId, frame) => this.handleHostFrame(frame));
	}

	get publicKey(): string {
		return this.keypair.publicKey;
	}

	get capsule(): RemoteChildAdmissionCapsule | undefined {
		return this._capsule;
	}

	get lease(): RemoteChildLease | undefined {
		return this._lease;
	}

	get startedModel(): boolean {
		return this._startedModel;
	}

	get terminalState(): RemoteChildState | undefined {
		return this._terminalState;
	}

	get commands(): readonly RemoteChildParentCommand[] {
		return this.receivedCommands;
	}

	private nextSeq(): number {
		this.seq += 1;
		return this.seq;
	}

	/** Admit via the invitation; resolves only with a committed ADMITTED. */
	admit(params: MockRemoteWorkerAdmitParams): Promise<RemoteChildAdmissionCapsule> {
		const nonce = randomUUID();
		const admissionRequestId = params.admissionRequestId;
		const promise = new Promise<RemoteChildAdmissionCapsule>((resolve, reject) => {
			this.pendingAdmissions.set(admissionRequestId, { resolve, reject });
		});
		this.state = "admitting";
		this.transport.sendToHost(admissionRequestId, {
			type: "admit",
			attempt: 1,
			connectionEpoch: 1,
			sequence: 0,
			idempotencyKey: admissionRequestId,
			invitationId: params.invitationId,
			nonce,
			admissionRequestId,
			supportedVersions: params.supportedVersions ?? [REMOTE_CHILD_PROTOCOL],
			workloadClaims: params.workloadClaims,
			workerPublicKey: this.keypair.publicKey,
			possessionSignature: signPossession(this.keypair.privateKey, this.keypair.publicKey, nonce),
		});
		return promise;
	}

	/** Attach (or re-attach) with the current lease; resolves with the renewed lease. */
	attach(): Promise<RemoteChildLease> {
		const capsule = this._capsule;
		const lease = this._lease;
		if (!capsule || !lease) throw new Error("worker has no capsule or lease to attach with");
		const key = `attach-${this.nextSeq()}`;
		const promise = new Promise<RemoteChildLease>((resolve, reject) => {
			this.pendingAttaches.set(key, { resolve, reject });
		});
		this.transport.sendToHost(capsule.childId, {
			type: "attach",
			attempt: capsule.attempt,
			connectionEpoch: lease.connectionEpoch,
			sequence: this.seq,
			idempotencyKey: key,
			leaseId: lease.leaseId,
			workerPublicKey: this.keypair.publicKey,
			possessionSignature: signPossession(this.keypair.privateKey, this.keypair.publicKey, lease.leaseId),
		});
		return promise;
	}

	/** The admission barrier: refuse to run until a committed ADMITTED exists. */
	startModel(): void {
		if (this.state !== "admitted" && this.state !== "attached" && this.state !== "running") {
			throw new Error("startModel() called before a committed ADMITTED was received");
		}
		this._startedModel = true;
	}

	startAck(): void {
		const capsule = this.requireCapsule();
		this.sendMutation(capsule.childId, {
			type: "start_ack",
			attempt: capsule.attempt,
			connectionEpoch: this.currentEpoch(),
			sequence: this.nextSeq(),
			idempotencyKey: `start-ack-${this.seq}`,
		});
		this.state = "running";
	}

	heartbeat(eventSeq: number, activitySummary?: string): Promise<void> {
		const capsule = this.requireCapsule();
		const key = `heartbeat-${this.nextSeq()}`;
		const promise = new Promise<void>((resolve, reject) => {
			this.pendingHeartbeats.set(key, { resolve, reject });
		});
		this.sendMutation(capsule.childId, {
			type: "heartbeat",
			attempt: capsule.attempt,
			connectionEpoch: this.currentEpoch(),
			sequence: this.seq,
			idempotencyKey: key,
			eventSeq,
			activitySummary,
		});
		return promise;
	}

	checkpoint(ref: string, hash: string, checkpointSeq: number): Promise<void> {
		const capsule = this.requireCapsule();
		const key = `checkpoint-${this.nextSeq()}`;
		const promise = new Promise<void>((resolve, reject) => {
			this.pendingCheckpoints.set(key, { resolve, reject });
		});
		this.sendMutation(capsule.childId, {
			type: "checkpoint",
			attempt: capsule.attempt,
			connectionEpoch: this.currentEpoch(),
			sequence: this.seq,
			idempotencyKey: key,
			checkpointRef: ref,
			checkpointHash: hash,
			checkpointSeq,
		});
		return promise;
	}

	sendMessage(text: string): Promise<{ status: string; messageId?: string }> {
		const capsule = this.requireCapsule();
		const key = `message-${this.nextSeq()}`;
		const promise = new Promise<{ status: string; messageId?: string }>((resolve, reject) => {
			this.pendingMessages.set(key, { resolve, reject });
		});
		this.sendMutation(capsule.childId, {
			type: "message",
			attempt: capsule.attempt,
			connectionEpoch: this.currentEpoch(),
			sequence: this.seq,
			idempotencyKey: key,
			text,
		});
		return promise;
	}

	complete(result?: string): void {
		const capsule = this.requireCapsule();
		this.sendMutation(capsule.childId, {
			type: "complete",
			attempt: capsule.attempt,
			connectionEpoch: this.currentEpoch(),
			sequence: this.nextSeq(),
			idempotencyKey: `complete-${this.seq}`,
			result,
		});
	}

	fail(reason: string): void {
		const capsule = this.requireCapsule();
		this.sendMutation(capsule.childId, {
			type: "fail",
			attempt: capsule.attempt,
			connectionEpoch: this.currentEpoch(),
			sequence: this.nextSeq(),
			idempotencyKey: `fail-${this.seq}`,
			reason,
		});
	}

	ackCommand(commandSeq: number, acknowledged = true): void {
		const capsule = this.requireCapsule();
		this.sendMutation(capsule.childId, {
			type: "command_ack",
			attempt: capsule.attempt,
			connectionEpoch: this.currentEpoch(),
			sequence: this.nextSeq(),
			idempotencyKey: `ack-${commandSeq}-${this.seq}`,
			commandSeq,
			acknowledged,
		});
	}

	private requireCapsule(): RemoteChildAdmissionCapsule {
		if (!this._capsule) throw new Error("worker is not admitted");
		return this._capsule;
	}

	private currentEpoch(): number {
		return this._lease?.connectionEpoch ?? this._capsule?.connectionEpoch ?? 1;
	}

	private sendMutation(childId: string, frame: Parameters<RemoteChildTransport["sendToHost"]>[1]): void {
		this.transport.sendToHost(childId, frame);
	}

	private handleHostFrame(frame: RemoteChildHostFrame): void {
		switch (frame.type) {
			case "admitted": {
				const pending = this.pendingAdmissions.get(frame.admissionRequestId);
				if (!pending) return;
				this.pendingAdmissions.delete(frame.admissionRequestId);
				this._capsule = frame.capsule;
				this._lease = frame.capsule.lease;
				this.state = "admitted";
				pending.resolve(frame.capsule);
				return;
			}
			case "admission_denied": {
				const pending = this.pendingAdmissions.get(frame.admissionRequestId);
				if (!pending) return;
				this.pendingAdmissions.delete(frame.admissionRequestId);
				this.state = "idle";
				pending.reject(new RemoteChildDeniedError(frame.reason));
				return;
			}
			case "lease_renewed": {
				this._lease = frame.lease;
				if (this.state === "admitted" || this.state === "admitting") this.state = "attached";
				for (const pending of this.pendingAttaches.values()) pending.resolve(frame.lease);
				this.pendingAttaches.clear();
				for (const pending of this.pendingHeartbeats.values()) pending.resolve();
				this.pendingHeartbeats.clear();
				return;
			}
			case "lease_expired": {
				this._terminalState = "lease_expired";
				this.state = "terminal";
				return;
			}
			case "parent_command": {
				this.receivedCommands.push(frame.command);
				this.onCommand?.(frame.command);
				if (this.autoAckCommands) this.ackCommand(frame.command.commandSeq, true);
				return;
			}
			case "parent_command_receipt": {
				return;
			}
			case "message_receipt": {
				for (const pending of this.pendingMessages.values()) {
					pending.resolve({ status: frame.status, messageId: frame.messageId });
				}
				this.pendingMessages.clear();
				return;
			}
			case "checkpoint_receipt": {
				if (frame.status === "accepted") {
					for (const pending of this.pendingCheckpoints.values()) pending.resolve();
				} else {
					for (const pending of this.pendingCheckpoints.values()) {
						pending.reject(new Error(frame.reason ?? "checkpoint rejected"));
					}
				}
				this.pendingCheckpoints.clear();
				return;
			}
			case "terminal_notice": {
				this._terminalState = frame.state;
				this.state = "terminal";
				return;
			}
			case "error": {
				return;
			}
		}
	}
}
