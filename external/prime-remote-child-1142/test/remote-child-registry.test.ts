import { mkdirSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { Agent } from "@earendil-works/pi-agent-core";
import { type AssistantMessage, createAssistantMessageEventStream, getModel, type Usage } from "@earendil-works/pi-ai";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { AgentSession } from "../src/core/agent-session.js";
import { AuthStorage } from "../src/core/auth-storage.js";
import { convertToLlm } from "../src/core/messages.js";
import { ModelRegistry } from "../src/core/model-registry.js";
import { SessionManager } from "../src/core/session-manager.js";
import { SettingsManager } from "../src/core/settings-manager.js";
import { RemoteChildAdmissionHost } from "../src/remote-child/admission.js";
import { createRemoteChildRegistryProvider } from "../src/remote-child/registry.js";
import { InMemoryRemoteChildTransport } from "../src/remote-child/transport.js";
import { MockRemoteWorker } from "../src/remote-child/worker.js";
import { createTestResourceLoader } from "./utilities.js";

const model = getModel("anthropic", "claude-sonnet-4-5")!;
const WORKLOAD = { ticket: "1142" };

function usage(): Usage {
	return {
		input: 3,
		output: 1,
		cacheRead: 0,
		cacheWrite: 0,
		totalTokens: 4,
		cost: { input: 3, output: 1, cacheRead: 0, cacheWrite: 0, total: 4 },
	};
}

function assistantMessage(text: string): AssistantMessage {
	return {
		role: "assistant",
		content: [{ type: "text", text }],
		api: model.api,
		provider: model.provider,
		model: model.id,
		usage: usage(),
		stopReason: "stop",
		timestamp: Date.now(),
	};
}

function streamAnswer(text: string): ReturnType<typeof createAssistantMessageEventStream> {
	const stream = createAssistantMessageEventStream();
	queueMicrotask(() => {
		stream.push({ type: "done", reason: "stop", message: assistantMessage(text) });
	});
	return stream;
}

describe("remote-child registry integration", () => {
	let tempDir: string;
	const sessions: AgentSession[] = [];

	beforeEach(() => {
		tempDir = join(tmpdir(), `pi-remote-child-${Date.now()}-${Math.random().toString(36).slice(2)}`);
		mkdirSync(tempDir, { recursive: true });
	});

	afterEach(() => {
		for (const session of sessions) session.dispose();
		sessions.length = 0;
		rmSync(tempDir, { recursive: true, force: true });
	});

	function createParentSession(): AgentSession {
		const authStorage = AuthStorage.create(join(tempDir, "auth.json"));
		authStorage.setRuntimeApiKey("anthropic", "test-key");
		const sessionManager = SessionManager.create(tempDir, join(tempDir, "sessions"));
		const settingsManager = SettingsManager.create(tempDir, tempDir);
		const agent = new Agent({
			convertToLlm,
			getApiKey: () => "test-key",
			initialState: { model, systemPrompt: "", tools: [], thinkingLevel: "off" },
			streamFn: (_m, _c) => streamAnswer("ok"),
		});
		const session = new AgentSession({
			agent,
			sessionManager,
			settingsManager,
			cwd: tempDir,
			modelRegistry: ModelRegistry.create(authStorage, join(tempDir, "models.json")),
			resourceLoader: createTestResourceLoader(),
			rlmDepth: 0,
			rlmMaxDepth: 4,
		});
		sessions.push(session);
		return session;
	}

	function wireHost(host: RemoteChildAdmissionHost, transport: InMemoryRemoteChildTransport): void {
		transport.onWorkerFrame((childId, frame) => {
			void host.dispatchWorkerFrame(childId, frame);
		});
	}

	it("lists an admitted remote worker in the inviting old parent session, not a sibling parent", async () => {
		const parent = createParentSession();
		const parentSessionFile = parent.sessionFile ?? join(tempDir, "sessions", `${parent.sessionId}.jsonl`);
		const transport = new InMemoryRemoteChildTransport();
		const host = new RemoteChildAdmissionHost({
			storeDir: join(tempDir, "remote-store"),
			parentSessionId: parent.sessionId,
			parentSessionFile,
			depth: parent.rlmDepth,
			transport,
		});
		wireHost(host, transport);
		parent.setRemoteChildRegistryProvider(createRemoteChildRegistryProvider(host));

		const inv = host.invite({
			parentSessionId: parent.sessionId,
			parentSessionFile,
			depth: parent.rlmDepth,
			name: "remote-worker",
			workload: WORKLOAD,
		});
		const worker = new MockRemoteWorker({ transport });
		const capsule = await worker.admit({
			invitationId: inv.invitationId,
			admissionRequestId: "admit-1",
			workloadClaims: WORKLOAD,
		});
		await worker.attach();
		worker.startAck();

		const listed = await parent.listRlmSubagents();
		expect(listed.subagents).toHaveLength(1);
		expect(listed.subagents[0]).toMatchObject({
			rlm_child_id: capsule.childId,
			session_id: capsule.sessionId,
			runtime: "remote",
			remote_state: "running",
			attempt: 1,
		});

		const siblingParent = createParentSession();
		const siblingResult = await siblingParent.listRlmSubagents();
		expect(siblingResult.subagents).toHaveLength(0);

		host.close();
		transport.close();
	});

	it("keeps default list behavior unchanged when no provider is installed", async () => {
		const parent = createParentSession();
		const result = await parent.listRlmSubagents();
		expect(result.subagents).toEqual([]);
	});

	it("routes a worker message back to the inviting parent, not a sibling parent", async () => {
		const parent = createParentSession();
		const parentSessionFile = parent.sessionFile ?? join(tempDir, "sessions", `${parent.sessionId}.jsonl`);
		const transport = new InMemoryRemoteChildTransport();
		const host = new RemoteChildAdmissionHost({
			storeDir: join(tempDir, "remote-store"),
			parentSessionId: parent.sessionId,
			parentSessionFile,
			depth: parent.rlmDepth,
			transport,
			onWorkerMessage: (message) => {
				void parent.acceptAgentMessagePrompt(`[from child:remote-worker] ${message.text}`);
			},
		});
		wireHost(host, transport);
		parent.setRemoteChildRegistryProvider(createRemoteChildRegistryProvider(host));

		const inv = host.invite({
			parentSessionId: parent.sessionId,
			parentSessionFile,
			depth: parent.rlmDepth,
			name: "remote-worker",
			workload: WORKLOAD,
		});
		const worker = new MockRemoteWorker({ transport });
		await worker.admit({ invitationId: inv.invitationId, admissionRequestId: "admit-1", workloadClaims: WORKLOAD });
		await worker.attach();
		worker.startAck();
		const receipt = await worker.sendMessage("slice result is ready");
		expect(receipt.status).toBe("delivered");

		await vi.waitFor(() => {
			expect(parent.messages.some((message) => JSON.stringify(message).includes("slice result is ready"))).toBe(
				true,
			);
		});

		const siblingParent = createParentSession();
		expect(siblingParent.messages.some((message) => JSON.stringify(message).includes("slice result is ready"))).toBe(
			false,
		);

		host.close();
		transport.close();
	});
});
