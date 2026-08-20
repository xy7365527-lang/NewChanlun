/**
 * Transport seam for prime-agent.remote-child/v1.
 *
 * The application protocol (admission, family messaging, lifecycle,
 * capability semantics) is layered over a pluggable transport. The Core MVP
 * runs over this in-memory transport: bounded, deterministic, no port, no
 * credential, no broker, no inbound tunnel. A production transport (for
 * example an outbound broker) is a deployment decision, not a core dependency.
 */
import type { RemoteChildHostFrame, RemoteChildWorkerFrame } from "./types.js";

export type RemoteChildTransportDirection = "toHost" | "toWorker";

export type RemoteChildTransportFault =
	| { kind: "drop"; direction: RemoteChildTransportDirection; count: number }
	| { kind: "duplicate"; direction: RemoteChildTransportDirection; count: number };

export interface RemoteChildTransport {
	/** Worker side: send a frame toward the host. */
	sendToHost(childId: string, frame: RemoteChildWorkerFrame): void;
	/** Worker side: subscribe to frames from the host. */
	onHostFrame(handler: (childId: string, frame: RemoteChildHostFrame) => void): () => void;
	/** Host side: send a frame toward a worker. */
	sendToWorker(childId: string, frame: RemoteChildHostFrame): void;
	/** Host side: subscribe to frames arriving from workers. */
	onWorkerFrame(handler: (childId: string, frame: RemoteChildWorkerFrame) => void): () => void;
	/** Simulate a network partition in one direction (frames are dropped). */
	disconnect(direction: RemoteChildTransportDirection): void;
	reconnect(direction: RemoteChildTransportDirection): void;
	/** Inject a deterministic fault consumed by the next frames in a direction. */
	injectFault(fault: RemoteChildTransportFault): void;
	/** Count of frames dropped by fault injection or partition. */
	readonly droppedToHost: number;
	readonly droppedToWorker: number;
	close(): void;
}

export class InMemoryRemoteChildTransport implements RemoteChildTransport {
	private readonly hostHandlers = new Set<(childId: string, frame: RemoteChildWorkerFrame) => void>();
	private readonly workerHandlers = new Set<(childId: string, frame: RemoteChildHostFrame) => void>();
	private readonly faults = new Map<RemoteChildTransportDirection, RemoteChildTransportFault[]>();
	private disconnectedToHost = false;
	private disconnectedToWorker = false;
	private _droppedToHost = 0;
	private _droppedToWorker = 0;

	sendToHost(childId: string, frame: RemoteChildWorkerFrame): void {
		if (this.disconnectedToHost) {
			this._droppedToHost += 1;
			return;
		}
		this.deliverWorkerFrame(childId, frame, () => this.hostHandlers);
	}

	onHostFrame(handler: (childId: string, frame: RemoteChildHostFrame) => void): () => void {
		this.workerHandlers.add(handler);
		return () => this.workerHandlers.delete(handler);
	}

	sendToWorker(childId: string, frame: RemoteChildHostFrame): void {
		if (this.disconnectedToWorker) {
			this._droppedToWorker += 1;
			return;
		}
		this.deliverHostFrame(childId, frame, () => this.workerHandlers);
	}

	onWorkerFrame(handler: (childId: string, frame: RemoteChildWorkerFrame) => void): () => void {
		this.hostHandlers.add(handler);
		return () => this.hostHandlers.delete(handler);
	}

	disconnect(direction: RemoteChildTransportDirection): void {
		if (direction === "toHost") this.disconnectedToHost = true;
		else this.disconnectedToWorker = true;
	}

	reconnect(direction: RemoteChildTransportDirection): void {
		if (direction === "toHost") this.disconnectedToHost = false;
		else this.disconnectedToWorker = false;
	}

	injectFault(fault: RemoteChildTransportFault): void {
		const list = this.faults.get(fault.direction) ?? [];
		list.push(fault);
		this.faults.set(fault.direction, list);
	}

	get droppedToHost(): number {
		return this._droppedToHost;
	}

	get droppedToWorker(): number {
		return this._droppedToWorker;
	}

	close(): void {
		this.hostHandlers.clear();
		this.workerHandlers.clear();
		this.faults.clear();
		this.disconnectedToHost = true;
		this.disconnectedToWorker = true;
	}

	private takeFault(direction: RemoteChildTransportDirection): "drop" | "duplicate" | "none" {
		const list = this.faults.get(direction);
		if (!list || list.length === 0) return "none";
		const fault = list[0]!;
		fault.count -= 1;
		if (fault.count <= 0) list.shift();
		return fault.kind;
	}

	private deliverWorkerFrame(
		childId: string,
		frame: RemoteChildWorkerFrame,
		handlers: () => Set<(childId: string, frame: RemoteChildWorkerFrame) => void>,
	): void {
		const disposition = this.takeFault("toHost");
		if (disposition === "drop") {
			this._droppedToHost += 1;
			return;
		}
		for (const handler of [...handlers()]) handler(childId, frame);
		if (disposition === "duplicate") {
			for (const handler of [...handlers()]) handler(childId, frame);
		}
	}

	private deliverHostFrame(
		childId: string,
		frame: RemoteChildHostFrame,
		handlers: () => Set<(childId: string, frame: RemoteChildHostFrame) => void>,
	): void {
		const disposition = this.takeFault("toWorker");
		if (disposition === "drop") {
			this._droppedToWorker += 1;
			return;
		}
		for (const handler of [...handlers()]) handler(childId, frame);
		if (disposition === "duplicate") {
			for (const handler of [...handlers()]) handler(childId, frame);
		}
	}
}
