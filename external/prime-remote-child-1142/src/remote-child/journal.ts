/**
 * Durable append-only journal for remote-child state.
 *
 * One JSONL file per parent session. Every mutation is a single small
 * O_APPEND write followed by fsync, so the journal survives a crash and
 * reproduces invitations, children, parent commands, and tombstones.
 * Replay is fail-closed: an unknown op, a malformed record, or an
 * out-of-contract field aborts loading instead of silently dropping state.
 */
import { closeSync, existsSync, fsyncSync, openSync, readFileSync, statSync, writeSync } from "node:fs";
import { dirname } from "node:path";
import type { RemoteChildInvitationRecord, RemoteChildParentCommand, RemoteChildRecord } from "./types.js";
import { REMOTE_CHILD_MAX_BUFFERED_FRAMES } from "./types.js";

export const REMOTE_CHILD_JOURNAL_MAX_BYTES = 64 * 1024 * 1024;

export type RemoteChildJournalRecord =
	| { v: 1; op: "meta"; at: number; sessionsDir: string }
	| { v: 1; op: "invitation"; at: number; record: RemoteChildInvitationRecord }
	| { v: 1; op: "child"; at: number; record: RemoteChildRecord }
	| { v: 1; op: "command"; at: number; childId: string; command: RemoteChildParentCommand }
	| { v: 1; op: "delete"; at: number; childId: string; reason: string };

export interface RemoteChildJournalReplay {
	invitations: Map<string, RemoteChildInvitationRecord>;
	children: Map<string, RemoteChildRecord>;
	commands: Map<string, RemoteChildParentCommand>;
	deleted: Set<string>;
}

const KNOWN_OPS = new Set(["meta", "invitation", "child", "command", "delete"]);

function isFiniteNumber(value: unknown): value is number {
	return typeof value === "number" && Number.isFinite(value);
}

function isNonEmptyString(value: unknown): value is string {
	return typeof value === "string" && value.length > 0;
}

function commandKey(childId: string, commandSeq: number): string {
	return `${childId}:${commandSeq}`;
}

function validateRecord(record: RemoteChildJournalRecord): void {
	if (!isFiniteNumber(record.at)) throw new Error("journal record has a non-finite timestamp");
	switch (record.op) {
		case "meta":
			if (!isNonEmptyString(record.sessionsDir)) throw new Error("journal meta record is missing sessionsDir");
			return;
		case "invitation":
			if (!isNonEmptyString(record.record.invitationHash))
				throw new Error("journal invitation record is missing its hash");
			if (!isNonEmptyString(record.record.parentSessionId)) {
				throw new Error("journal invitation record is missing parentSessionId");
			}
			return;
		case "child":
			if (!isNonEmptyString(record.record.childId) || !isNonEmptyString(record.record.sessionId)) {
				throw new Error("journal child record is missing an identity");
			}
			if (!isNonEmptyString(record.record.parentSessionId))
				throw new Error("journal child record is missing parentSessionId");
			if (!Number.isInteger(record.record.attempt) || record.record.attempt < 1) {
				throw new Error("journal child record has an invalid attempt");
			}
			if (!Number.isInteger(record.record.connectionEpoch) || record.record.connectionEpoch < 1) {
				throw new Error("journal child record has an invalid connection epoch");
			}
			return;
		case "command":
			if (!isNonEmptyString(record.childId)) throw new Error("journal command record is missing childId");
			if (!Number.isInteger(record.command.commandSeq) || record.command.commandSeq < 1) {
				throw new Error("journal command record has an invalid sequence");
			}
			return;
		case "delete":
			if (!isNonEmptyString(record.childId)) throw new Error("journal delete record is missing childId");
			return;
	}
}

function parseRecord(line: string): RemoteChildJournalRecord {
	let parsed: unknown;
	try {
		parsed = JSON.parse(line);
	} catch {
		throw new Error("journal contains a non-JSON line");
	}
	if (typeof parsed !== "object" || parsed === null) throw new Error("journal record is not an object");
	const record = parsed as RemoteChildJournalRecord;
	if (record.v !== 1) throw new Error("journal record has an unsupported version");
	if (typeof record.op !== "string" || !KNOWN_OPS.has(record.op)) {
		throw new Error(`journal record has an unknown op: ${String(record.op)}`);
	}
	validateRecord(record);
	return record;
}

export function replayJournalRecords(records: readonly RemoteChildJournalRecord[]): RemoteChildJournalReplay {
	const result: RemoteChildJournalReplay = {
		invitations: new Map(),
		children: new Map(),
		commands: new Map(),
		deleted: new Set(),
	};
	for (const record of records) {
		switch (record.op) {
			case "meta":
				break;
			case "invitation":
				result.invitations.set(record.record.invitationHash, record.record);
				break;
			case "child":
				result.children.set(record.record.childId, record.record);
				break;
			case "command":
				result.commands.set(commandKey(record.childId, record.command.commandSeq), record.command);
				break;
			case "delete":
				result.deleted.add(record.childId);
				break;
		}
	}
	return result;
}

export class RemoteChildJournal {
	private fd: number;
	private bytes = 0;
	private closed = false;

	private constructor(
		private readonly filePath: string,
		fd: number,
		bytes: number,
	) {
		this.fd = fd;
		this.bytes = bytes;
	}

	static open(filePath: string): RemoteChildJournal {
		const parent = dirname(filePath);
		if (!existsSync(parent)) {
			throw new Error(`journal directory does not exist: ${parent}`);
		}
		const fd = openSync(filePath, "a");
		try {
			const bytes = statSync(filePath).size;
			if (bytes > REMOTE_CHILD_JOURNAL_MAX_BYTES) {
				closeSync(fd);
				throw new Error(`journal exceeds ${REMOTE_CHILD_JOURNAL_MAX_BYTES} bytes; failing closed`);
			}
			return new RemoteChildJournal(filePath, fd, bytes);
		} catch (error) {
			closeSync(fd);
			throw error;
		}
	}

	append(record: RemoteChildJournalRecord): void {
		const line = `${JSON.stringify(record)}\n`;
		const buffer = Buffer.from(line, "utf8");
		let written = 0;
		while (written < buffer.length) {
			const count = writeSync(this.fd, buffer, written, buffer.length - written, null);
			if (count <= 0) throw new Error("journal append made no progress");
			written += count;
		}
		fsyncSync(this.fd);
		this.bytes += buffer.length;
		if (this.bytes > REMOTE_CHILD_JOURNAL_MAX_BYTES) {
			throw new Error(`journal exceeds ${REMOTE_CHILD_JOURNAL_MAX_BYTES} bytes; failing closed`);
		}
	}

	/** Re-read the file from disk; used by restart/reload paths and tests. */
	replay(): RemoteChildJournalReplay {
		const buffer = readFileSync(this.filePath, "utf8");
		if (Buffer.byteLength(buffer, "utf8") > REMOTE_CHILD_JOURNAL_MAX_BYTES) {
			throw new Error(`journal exceeds ${REMOTE_CHILD_JOURNAL_MAX_BYTES} bytes; failing closed`);
		}
		const lines = buffer.split("\n").filter((line) => line.length > 0);
		const records: RemoteChildJournalRecord[] = [];
		for (const line of lines) {
			records.push(parseRecord(line));
		}
		const replay = replayJournalRecords(records);
		// A child record that has no matching allocation would be a ghost; the
		// allocation is carried on the same record, so this is a no-op guard.
		// The buffered-frame bound is the only external cap we mirror here.
		if (replay.children.size > REMOTE_CHILD_MAX_BUFFERED_FRAMES) {
			throw new Error("journal holds more children than the protocol bound permits");
		}
		return replay;
	}

	close(): void {
		if (this.closed) return;
		this.closed = true;
		closeSync(this.fd);
	}
}

export function commandKeyFor(childId: string, commandSeq: number): string {
	return commandKey(childId, commandSeq);
}
