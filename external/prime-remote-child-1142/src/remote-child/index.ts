/**
 * prime-agent.remote-child/v1 — Core MVP.
 *
 * Layered over a pluggable transport and independent of the local daemon
 * protocol, model provider credentials, brokers, and inbound tunnels.
 */

export type { RemoteChildAdmissionHostOptions, RemoteChildInviteOutcome } from "./admission.js";
export { RemoteChildAdmissionHost } from "./admission.js";
export type { Ed25519Keypair } from "./crypto.js";
export {
	generateCapabilityKeypair,
	generateOpaqueToken,
	publicKeyHash,
	sha256Hex,
	signPossession,
	verifyPossession,
} from "./crypto.js";
export type { RemoteChildJournalRecord, RemoteChildJournalReplay } from "./journal.js";
export { RemoteChildJournal, replayJournalRecords } from "./journal.js";
export { createRemoteChildRegistryProvider } from "./registry.js";
export type { RemoteChildTransport, RemoteChildTransportFault } from "./transport.js";
export { InMemoryRemoteChildTransport } from "./transport.js";
export type {
	RemoteChildAdmissionCapsule,
	RemoteChildAdmissionOutcome,
	RemoteChildAttachOutcome,
	RemoteChildCommandReceipt,
	RemoteChildHostFrame,
	RemoteChildInvitationRecord,
	RemoteChildInvitationSpec,
	RemoteChildLease,
	RemoteChildParentCommand,
	RemoteChildRecord,
	RemoteChildRegistryEntry,
	RemoteChildState,
	RemoteChildWorkerFrame,
	RemoteChildWorkerMessage,
	RemoteChildWorkloadClaims,
} from "./types.js";
export {
	isRemoteChildTerminal,
	REMOTE_CHILD_ACTIVE_STATES,
	REMOTE_CHILD_PROTOCOL,
	REMOTE_CHILD_TERMINAL_STATES,
} from "./types.js";
export type { MockRemoteWorkerAdmitParams, MockRemoteWorkerOptions } from "./worker.js";
export { MockRemoteWorker, RemoteChildDeniedError } from "./worker.js";
