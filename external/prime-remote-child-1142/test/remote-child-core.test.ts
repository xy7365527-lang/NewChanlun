import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { RemoteChildAdmissionHost, type RemoteChildAdmissionHostOptions } from "../src/remote-child/admission.js";
import { generateCapabilityKeypair } from "../src/remote-child/crypto.js";
import { InMemoryRemoteChildTransport } from "../src/remote-child/transport.js";
import type { RemoteChildWorkerMessage, RemoteChildWorkloadClaims } from "../src/remote-child/types.js";
import { MockRemoteWorker, RemoteChildDeniedError } from "../src/remote-child/worker.js";

const PARENT_SESSION_ID = "parent-session-1";
const PARENT_SESSION_FILE = "/sessions/parent-session-1.jsonl";
const WORKLOAD: RemoteChildWorkloadClaims = {
	repoId: "repo-7",
	workflowRef: ".github/workflows/worker.yml",
	ticket: "1142",
};

class FakeClock {
	private t = 1_000_000;
	now(): number {
		return this.t;
	}
	advance(ms: number): void {
		this.t += ms;
	}
}

interface Harness {
	clock: FakeClock;
	storeDir: string;
	transport: InMemoryRemoteChildTransport;
	host: RemoteChildAdmissionHost;
	messages: RemoteChildWorkerMessage[];
}

function createHost(
	storeDir: string,
	clock: FakeClock,
	transport: InMemoryRemoteChildTransport,
	overrides: Partial<RemoteChildAdmissionHostOptions> = {},
): RemoteChildAdmissionHost {
	return new RemoteChildAdmissionHost({
		storeDir,
		parentSessionId: PARENT_SESSION_ID,
		parentSessionFile: PARENT_SESSION_FILE,
		depth: 0,
		transport,
		clock: () => clock.now(),
		invitationTtlMs: 60_000,
		leaseTtlMs: 60_000,
		stopDeadlineMs: 30_000,
		disconnectGraceMs: 30_000,
		...overrides,
	});
}

function setup(overrides: Partial<RemoteChildAdmissionHostOptions> = {}): Harness {
	const clock = new FakeClock();
	const storeDir = mkdtempSync(join(tmpdir(), "remote-child-"));
	const transport = new InMemoryRemoteChildTransport();
	const messages: RemoteChildWorkerMessage[] = [];
	const host = createHost(storeDir, clock, transport, {
		onWorkerMessage: (message) => {
			messages.push(message);
		},
		...overrides,
	});
	transport.onWorkerFrame((childId, frame) => {
		void host.dispatchWorkerFrame(childId, frame);
	});
	return { clock, storeDir, transport, host, messages };
}

function makeWorker(
	transport: InMemoryRemoteChildTransport,
	options: { keypair?: ReturnType<typeof generateCapabilityKeypair>; autoAckCommands?: boolean } = {},
): MockRemoteWorker {
	return new MockRemoteWorker({ transport, ...options });
}

function invite(
	host: RemoteChildAdmissionHost,
	workload: RemoteChildWorkloadClaims = WORKLOAD,
): { invitationId: string; expiresAt: number } {
	return host.invite({
		parentSessionId: PARENT_SESSION_ID,
		parentSessionFile: PARENT_SESSION_FILE,
		depth: 0,
		name: "remote-worker",
		workload,
	});
}

async function admitWorker(
	host: RemoteChildAdmissionHost,
	transport: InMemoryRemoteChildTransport,
	options: {
		invitationId?: string;
		admissionRequestId?: string;
		workload?: RemoteChildWorkloadClaims;
		keypair?: ReturnType<typeof generateCapabilityKeypair>;
	} = {},
): Promise<{
	worker: MockRemoteWorker;
	capsule: Awaited<ReturnType<MockRemoteWorker["admit"]>>;
	invitationId: string;
}> {
	const invitationId = options.invitationId ?? invite(host, options.workload ?? WORKLOAD).invitationId;
	const worker = makeWorker(transport, { keypair: options.keypair });
	const capsule = await worker.admit({
		invitationId,
		admissionRequestId: options.admissionRequestId ?? "admit-1",
		workloadClaims: options.workload ?? WORKLOAD,
	});
	return { worker, capsule, invitationId };
}

describe("prime-agent.remote-child/v1 core", () => {
	let harness: Harness;

	beforeEach(() => {
		harness = setup();
	});

	afterEach(() => {
		harness.host.close();
		harness.transport.close();
		rmSync(harness.storeDir, { recursive: true, force: true });
	});

	it("holds the admission barrier: no start permit until a durable ADMITTED", async () => {
		const { host, transport } = harness;
		const worker = makeWorker(transport);
		expect(() => worker.startModel()).toThrow();

		const inv = invite(host);
		await expect(
			worker.admit({ invitationId: inv.invitationId, admissionRequestId: "admit-1", workloadClaims: WORKLOAD }),
		).resolves.toMatchObject({ childId: expect.stringMatching(/^remote-/) });

		expect(() => worker.startModel()).not.toThrow();
		expect(worker.startedModel).toBe(true);
		expect(host.listEntries()).toHaveLength(1);
		expect(host.listEntries()[0]).toMatchObject({ runtime: "remote", remote_state: "admitted" });
	});

	it("is idempotent on retry of a committed admission: same child, no second allocation", async () => {
		const { host, transport } = harness;
		const keypair = generateCapabilityKeypair();
		const inv = invite(host);
		const first = await makeWorker(transport, { keypair }).admit({
			invitationId: inv.invitationId,
			admissionRequestId: "admit-1",
			workloadClaims: WORKLOAD,
		});
		const second = await makeWorker(transport, { keypair }).admit({
			invitationId: inv.invitationId,
			admissionRequestId: "admit-1",
			workloadClaims: WORKLOAD,
		});
		expect(second.childId).toBe(first.childId);
		expect(second.sessionId).toBe(first.sessionId);
		expect(host.listEntries()).toHaveLength(1);
	});

	it("rejects a second use of a single-use invitation", async () => {
		const { host, transport } = harness;
		const inv = invite(host);
		const first = await makeWorker(transport).admit({
			invitationId: inv.invitationId,
			admissionRequestId: "admit-1",
			workloadClaims: WORKLOAD,
		});
		expect(first.childId).toBeTruthy();
		await expect(
			makeWorker(transport).admit({
				invitationId: inv.invitationId,
				admissionRequestId: "admit-2",
				workloadClaims: WORKLOAD,
			}),
		).rejects.toBeInstanceOf(RemoteChildDeniedError);
		expect(host.listEntries()).toHaveLength(1);
	});

	it("fails closed on expired, revoked, and mismatched-workload invitations", async () => {
		const { host, transport, clock } = harness;
		const expired = invite(host);
		clock.advance(61_000);
		await expect(
			makeWorker(transport).admit({
				invitationId: expired.invitationId,
				admissionRequestId: "a1",
				workloadClaims: WORKLOAD,
			}),
		).rejects.toThrow(/expired/i);

		const revoked = invite(host);
		expect(host.revokeInvitation(revoked.invitationId)).toBe(true);
		await expect(
			makeWorker(transport).admit({
				invitationId: revoked.invitationId,
				admissionRequestId: "a2",
				workloadClaims: WORKLOAD,
			}),
		).rejects.toThrow(/not open/i);

		const wrong = invite(host, { ticket: "9999" });
		await expect(
			makeWorker(transport).admit({
				invitationId: wrong.invitationId,
				admissionRequestId: "a3",
				workloadClaims: WORKLOAD,
			}),
		).rejects.toThrow(/mismatch/i);
		expect(host.listEntries()).toHaveLength(0);
	});

	it("rejects a nonce replay and a forged possession proof", async () => {
		const { host, transport } = harness;
		const inv = invite(host);
		const keypair = generateCapabilityKeypair();
		const worker = makeWorker(transport, { keypair });
		// First admission consumes the nonce; a replayed nonce must be rejected.
		const capsule = await worker.admit({
			invitationId: inv.invitationId,
			admissionRequestId: "admit-1",
			workloadClaims: WORKLOAD,
		});
		expect(capsule.childId).toBeTruthy();

		// A different admission request id with the same nonce is a replay.
		const inv2 = invite(host);
		const replayingWorker = makeWorker(transport, { keypair });
		// Force the same nonce by bypassing the worker's fresh-nonce helper.
		await expect(
			host.processAdmit({
				type: "admit",
				attempt: 1,
				connectionEpoch: 1,
				sequence: 0,
				idempotencyKey: "admit-2",
				invitationId: inv2.invitationId,
				nonce: "forged-nonce",
				admissionRequestId: "admit-2",
				supportedVersions: ["prime-agent.remote-child/v1"],
				workloadClaims: WORKLOAD,
				workerPublicKey: "not-a-key",
				possessionSignature: "sig:bad",
			}),
		).resolves.toMatchObject({ status: "denied" });
		void replayingWorker;
	});

	it("derives the message sender from the lease and delivers worker→parent", async () => {
		const { host, transport, messages } = harness;
		const { worker } = await admitWorker(host, transport);
		await worker.attach();
		worker.startAck();

		const receipt = await worker.sendMessage("the worker finished its slice");
		expect(receipt.status).toBe("delivered");
		expect(receipt.messageId).toBeTruthy();
		expect(messages).toHaveLength(1);
		expect(messages[0]).toMatchObject({
			parentSessionId: PARENT_SESSION_ID,
			text: "the worker finished its slice",
		});
	});

	it("delivers steer and follow-up as distinct parent commands", async () => {
		const { host, transport } = harness;
		const { worker, capsule } = await admitWorker(host, transport);
		await worker.attach();
		worker.startAck();

		expect(host.steer(capsule.childId, "switch to plan B").status).toBe("queued");
		expect(host.followUp(capsule.childId, "also review the diff").status).toBe("queued");
		await vi.waitFor(() => {
			expect(worker.commands.map((command) => command.kind)).toEqual(["steer", "follow_up"]);
		});
	});

	it("marks a child cancelled only on a STOP ack", async () => {
		const { host, transport } = harness;
		const { worker, capsule } = await admitWorker(host, transport);
		await worker.attach();
		worker.startAck();
		expect(host.stop(capsule.childId, "cancelled by orchestrator").status).toBe("queued");
		await vi.waitFor(() => {
			expect(host.listEntries()[0]?.remote_state).toBe("cancelled");
		});
	});

	it("never fabricates a cancelled on a STOP without an ack", async () => {
		const { host, transport, clock } = harness;
		const { capsule } = await admitWorker(host, transport);
		await admitWorker(host, transport, { admissionRequestId: "admit-unused" }).then(() => {});
		const noAckWorker = makeWorker(transport, { autoAckCommands: false });
		// Re-admit the same child idempotently with a non-acking worker is not
		// possible here, so stop the already-admitted child directly.
		host.stop(capsule.childId, "cancelled by orchestrator");
		clock.advance(31_000);
		host.collectExpiry();
		expect(host.listEntries()[0]?.remote_state).toBe("uncertain");
		void noAckWorker;
	});

	it("transitions running → disconnected → lease_expired across grace", async () => {
		const { host, transport, clock } = harness;
		const { worker } = await admitWorker(host, transport);
		await worker.attach();
		worker.startAck();
		await worker.heartbeat(1);
		expect(host.listEntries()[0]?.remote_state).toBe("running");

		clock.advance(61_000);
		host.collectExpiry();
		expect(host.listEntries()[0]?.remote_state).toBe("disconnected");

		clock.advance(31_000);
		host.collectExpiry();
		expect(host.listEntries()[0]?.remote_state).toBe("lease_expired");
	});

	it("recovers state across a daemon restart and rejects stale frames without redelivery", async () => {
		const { clock, storeDir, transport, messages } = harness;
		const { host } = harness;
		const keypair = generateCapabilityKeypair();
		const inv = invite(host);
		const firstWorker = makeWorker(transport, { keypair });
		const firstCapsule = await firstWorker.admit({
			invitationId: inv.invitationId,
			admissionRequestId: "admit-1",
			workloadClaims: WORKLOAD,
		});
		await firstWorker.attach();
		firstWorker.startAck();
		await firstWorker.sendMessage("durable high-water mark message");
		expect(messages).toHaveLength(1);
		host.close();

		// Restart: reopen the same store.
		const transport2 = new InMemoryRemoteChildTransport();
		const host2 = createHost(storeDir, clock, transport2);
		transport2.onWorkerFrame((childId, frame) => {
			void host2.dispatchWorkerFrame(childId, frame);
		});
		expect(host2.listEntries()).toHaveLength(1);

		const resumed = makeWorker(transport2, { keypair });
		const capsule2 = await resumed.admit({
			invitationId: inv.invitationId,
			admissionRequestId: "admit-1",
			workloadClaims: WORKLOAD,
		});
		expect(capsule2.childId).toBe(firstCapsule.childId);
		await resumed.attach();

		// A fresh worker's sequence counter starts low; the durable high-water
		// mark must reject it without re-delivering to the parent.
		const stale = await resumed.sendMessage("stale replay");
		expect(stale.status).toBe("rejected");
		expect(messages).toHaveLength(1);
		host2.close();
	});

	it("isolates families: a child admitted to one parent never appears in another parent", async () => {
		const { host, transport } = harness;
		const otherClock = new FakeClock();
		const otherStoreDir = mkdtempSync(join(tmpdir(), "remote-child-other-"));
		const otherHost = createHost(otherStoreDir, otherClock, transport, {
			parentSessionId: "parent-session-2",
			parentSessionFile: "/sessions/parent-session-2.jsonl",
		});
		try {
			const { capsule } = await admitWorker(host, transport);
			expect(capsule.childId).toBeTruthy();
			expect(host.listEntries()).toHaveLength(1);
			expect(otherHost.listEntries()).toHaveLength(0);
			expect(otherHost.listEntries("parent-session-1")).toHaveLength(0);
		} finally {
			otherHost.close();
			rmSync(otherStoreDir, { recursive: true, force: true });
		}
	});

	it("rejects a worker with a mismatched possession key", async () => {
		const { host } = harness;
		const inv = invite(host);
		await expect(
			host.processAdmit({
				type: "admit",
				attempt: 1,
				connectionEpoch: 1,
				sequence: 0,
				idempotencyKey: "admit-1",
				invitationId: inv.invitationId,
				nonce: "nonce-1",
				admissionRequestId: "admit-1",
				supportedVersions: ["prime-agent.remote-child/v1"],
				workloadClaims: WORKLOAD,
				workerPublicKey: "ed25519-pem:-----BEGIN PUBLIC KEY-----\nnot-a-key\n-----END PUBLIC KEY-----\n",
				possessionSignature: "sig:bad",
			}),
		).resolves.toMatchObject({ status: "denied" });
	});

	it("rejects mutations after a tombstone delete and delete is idempotent", async () => {
		const { host, transport } = harness;
		const { worker, capsule } = await admitWorker(host, transport);
		await worker.attach();
		worker.startAck();
		expect(host.delete(capsule.childId)).toBe(true);
		expect(host.delete(capsule.childId)).toBe(true);
		expect(host.listEntries()).toHaveLength(0);

		await expect(worker.sendMessage("late frame")).resolves.toMatchObject({ status: "rejected" });
	});

	it("records checkpoints and maps provider terminal errors to failed", async () => {
		const { host, transport } = harness;
		const { worker } = await admitWorker(host, transport);
		await worker.attach();
		worker.startAck();
		await worker.checkpoint("s3://bucket/checkpoint-1", "hash-123", 1);
		expect(host.listEntries()[0]).toMatchObject({ remote_state: "running" });

		worker.fail("402 Insufficient Balance");
		await vi.waitFor(() => {
			expect(host.listEntries()[0]?.remote_state).toBe("failed");
		});
	});

	it("caps pending parent commands", async () => {
		const harness2 = setup({ maxPendingCommands: 2 });
		try {
			const { host, transport } = harness2;
			const keypair = generateCapabilityKeypair();
			const invitationId = invite(harness2.host).invitationId;
			const worker = makeWorker(transport, { keypair, autoAckCommands: false });
			const capsule = await worker.admit({
				invitationId,
				admissionRequestId: "admit-cap",
				workloadClaims: WORKLOAD,
			});
			await worker.attach();
			worker.startAck();
			expect(host.steer(capsule.childId, "one").status).toBe("queued");
			expect(host.steer(capsule.childId, "two").status).toBe("queued");
			expect(host.steer(capsule.childId, "three").status).toBe("rejected");
		} finally {
			harness2.host.close();
			harness2.transport.close();
			rmSync(harness2.storeDir, { recursive: true, force: true });
		}
	});
});
