import type { RlmRemoteChildRegistryEntry } from "../core/rlm-runtime.js";

/**
 * prime-agent.remote-child/v1 — protocol types.
 *
 * This module is the application protocol of the remote-child Core MVP
 * (#1142). It is versioned independently of the local daemon protocol and is
 * carried over a pluggable transport; the Core MVP runs over an in-memory
 * transport and assumes no broker, network port, inbound tunnel, or hosted
 * service exists. Capability identity is deliberately unrelated to model
 * provider credentials.
 */

export const REMOTE_CHILD_PROTOCOL = "prime-agent.remote-child/v1";

/** Bounded frame size; oversized frames are rejected before buffering. */
export const REMOTE_CHILD_MAX_FRAME_BYTES = 256 * 1024;

/** Matches the existing agent-message size limit so remote messaging does not diverge. */
export const REMOTE_CHILD_MAX_MESSAGE_CHARS = 16_384;

/** Per-child cap on un-acked parent commands (steer/follow-up/stop). */
export const REMOTE_CHILD_MAX_PENDING_PARENT_COMMANDS = 20;

/** Per-direction bound on buffered transport frames. */
export const REMOTE_CHILD_MAX_BUFFERED_FRAMES = 1_024;

/** Out-of-order tolerance window for mutation sequences. */
export const REMOTE_CHILD_REORDER_WINDOW = 32;

/** Hard cap on same-ticket retries (attempt increments). */
export const REMOTE_CHILD_MAX_ATTEMPTS = 8;

/** Per-parent cap on concurrent remote children. */
export const REMOTE_CHILD_MAX_CHILDREN_PER_PARENT = 32;

export const REMOTE_CHILD_DEFAULT_INVITATION_TTL_MS = 5 * 60_000;
export const REMOTE_CHILD_DEFAULT_LEASE_TTL_MS = 60_000;
export const REMOTE_CHILD_DEFAULT_STOP_DEADLINE_MS = 30_000;
export const REMOTE_CHILD_DEFAULT_DISCONNECT_GRACE_MS = 60_000;

/**
 * Workload identity claims validated before a child is allocated. These are
 * exact-match constraints baked into an invitation; a mismatch fails closed.
 */
export interface RemoteChildWorkloadClaims {
	/** e.g. GitHub repo id. */
	repoId?: string;
	/** e.g. workflow file ref. */
	workflowRef?: string;
	/** e.g. GitHub run id. */
	runId?: string;
	/** e.g. GitHub run attempt. */
	runAttempt?: string;
	/** e.g. NewChanlun ticket number. */
	ticket?: string;
}

export interface RemoteChildInvitationSpec {
	parentSessionId: string;
	parentSessionFile: string;
	/** Human-readable name for the eventual child (optional). */
	name?: string;
	workload: RemoteChildWorkloadClaims;
	ttlMs?: number;
	/** Depth of the eventual child (parent depth + 1). */
	depth: number;
}

export type RemoteChildInvitationState = "open" | "consumed" | "revoked" | "expired";

export interface RemoteChildInvitationRecord {
	/** SHA-256 of the invitation id — the only persisted form. */
	invitationHash: string;
	state: RemoteChildInvitationState;
	issuedAt: number;
	expiresAt: number;
	parentSessionId: string;
	parentSessionFile: string;
	depth: number;
	name?: string;
	workload: RemoteChildWorkloadClaims;
	/** Idempotency key of the admission request that consumed this invitation. */
	consumedBy?: string;
	consumedAt?: number;
}

export type RemoteChildState =
	| "admitted"
	| "running"
	| "disconnected"
	| "cancelling"
	| "cancelled"
	| "completed"
	| "uncertain"
	| "lease_expired"
	| "failed";

export const REMOTE_CHILD_TERMINAL_STATES: ReadonlySet<RemoteChildState> = new Set<RemoteChildState>([
	"cancelled",
	"completed",
	"uncertain",
	"lease_expired",
	"failed",
]);

export const REMOTE_CHILD_ACTIVE_STATES: ReadonlySet<RemoteChildState> = new Set<RemoteChildState>([
	"admitted",
	"running",
	"disconnected",
	"cancelling",
]);

/** Durable remote-child record; the topology edge still lives in the RLM ledger. */
export interface RemoteChildRecord {
	protocolVersion: string;
	childId: string;
	sessionId: string;
	parentSessionId: string;
	parentSessionFile: string;
	depth: number;
	name: string;
	workload: RemoteChildWorkloadClaims;
	state: RemoteChildState;
	attempt: number;
	connectionEpoch: number;
	/** SHA-256 of the worker Ed25519 public key that owns the current lease. */
	workerPublicKeyHash: string;
	issuedAt: number;
	admittedAt: number;
	startedAt?: number;
	lastHeartbeatAt?: number;
	lastEventSeq: number;
	/** Idempotency key of the admission request that created this child. */
	admissionRequestId: string;
	/** SHA-256 of the invitation id that admitted this child. */
	admissionInvitationHash: string;
	/** Pending parent→worker commands not yet acked. */
	parentCommandHighWater: number;
	acknowledgedCommandSeq: number;
	disconnectDeadlineAt?: number;
	terminalAt?: number;
	terminalReason?: string;
	checkpointRef?: string;
	checkpointHash?: string;
	checkpointSeq?: number;
}

/** A child lease: the post-admission capability held by the worker. */
export interface RemoteChildLease {
	leaseId: string;
	childId: string;
	sessionId: string;
	parentSessionId: string;
	attempt: number;
	connectionEpoch: number;
	expiresAt: number;
	workerPublicKeyHash: string;
}

export interface RemoteChildAdmissionCapsule {
	protocolVersion: string;
	childId: string;
	sessionId: string;
	parentSessionId: string;
	depth: number;
	attempt: number;
	connectionEpoch: number;
	lease: RemoteChildLease;
}

/** Kinds of worker-initiated mutations (mirrors the worker frame `type` field). */
export type RemoteChildMutationKind =
	| "admit"
	| "attach"
	| "heartbeat"
	| "checkpoint"
	| "message"
	| "complete"
	| "fail"
	| "command_ack"
	| "start_ack";

/** A mutation frame carries attempt/epoch/sequence fencing plus an idempotency key. */
export interface RemoteChildMutationHeader {
	attempt: number;
	connectionEpoch: number;
	sequence: number;
	idempotencyKey: string;
}

export type RemoteChildParentCommandKind = "steer" | "follow_up" | "stop";

export interface RemoteChildParentCommand {
	commandId: string;
	commandSeq: number;
	kind: RemoteChildParentCommandKind;
	text: string;
	reason?: string;
	issuedAt: number;
	deadlineAt?: number;
	/** Host-internal routing key; not part of the wire contract. */
	childIdForRouting?: string;
}

export type RemoteChildWorkerFrame =
	| ({ type: "admit" } & RemoteChildMutationHeader & {
				invitationId: string;
				nonce: string;
				admissionRequestId: string;
				supportedVersions: string[];
				workloadClaims: RemoteChildWorkloadClaims;
				/** Nonce signed by the worker's Ed25519 key to prove possession. */
				workerPublicKey: string;
				possessionSignature: string;
			})
	| ({ type: "attach" } & RemoteChildMutationHeader & {
				leaseId: string;
				workerPublicKey: string;
				possessionSignature: string;
			})
	| ({ type: "start_ack" } & RemoteChildMutationHeader)
	| ({ type: "heartbeat" } & RemoteChildMutationHeader & {
				eventSeq: number;
				activitySummary?: string;
			})
	| ({ type: "checkpoint" } & RemoteChildMutationHeader & {
				checkpointRef: string;
				checkpointHash: string;
				checkpointSeq: number;
			})
	| ({ type: "message" } & RemoteChildMutationHeader & {
				text: string;
			})
	| ({ type: "complete" } & RemoteChildMutationHeader & {
				result?: string;
			})
	| ({ type: "fail" } & RemoteChildMutationHeader & {
				/** Sanitized error summary (no secrets). */
				reason: string;
			})
	| ({ type: "command_ack" } & RemoteChildMutationHeader & {
				commandSeq: number;
				acknowledged: boolean;
			});

export type RemoteChildHostFrame =
	| {
			type: "admitted";
			capsule: RemoteChildAdmissionCapsule;
			/** Echoed admission idempotency key for crash-safe retry matching. */
			admissionRequestId: string;
	  }
	| { type: "admission_denied"; reason: string; admissionRequestId: string }
	| { type: "lease_renewed"; lease: RemoteChildLease }
	| { type: "lease_expired"; reason: string }
	| { type: "parent_command"; command: RemoteChildParentCommand }
	| {
			type: "parent_command_receipt";
			commandSeq: number;
			outcome: "queued" | "duplicate" | "rejected";
	  }
	| {
			type: "message_receipt";
			status: "delivered" | "duplicate" | "rejected";
			messageId?: string;
			reason?: string;
	  }
	| { type: "checkpoint_receipt"; status: "accepted" | "rejected"; reason?: string }
	| { type: "terminal_notice"; state: RemoteChildState; reason?: string }
	| { type: "error"; reason: string };

export type RemoteChildAdmissionOutcome =
	| { status: "admitted"; capsule: RemoteChildAdmissionCapsule }
	| { status: "denied"; reason: string };

export type RemoteChildAttachOutcome =
	| { status: "attached"; lease: RemoteChildLease }
	| { status: "denied"; reason: string };

/** Worker→parent message delivery result. */
export interface RemoteChildMessageReceipt {
	status: "delivered" | "duplicate";
	messageId: string;
}

export interface RemoteChildCommandReceipt {
	status: "queued" | "duplicate" | "rejected";
	commandSeq: number;
	reason?: string;
}

/** Parent-side registry entry; single source of truth lives in `rlm-runtime.ts`. */
export type RemoteChildRegistryEntry = RlmRemoteChildRegistryEntry;

/** Worker→parent message whose sender is derived by the host, never by payload. */
export interface RemoteChildWorkerMessage {
	childId: string;
	sessionId: string;
	parentSessionId: string;
	messageId: string;
	text: string;
	attempt: number;
	connectionEpoch: number;
	sequence: number;
}

export function isRemoteChildTerminal(state: RemoteChildState): boolean {
	return REMOTE_CHILD_TERMINAL_STATES.has(state);
}
