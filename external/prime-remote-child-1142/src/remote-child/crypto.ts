/**
 * Capability cryptography for prime-agent.remote-child/v1.
 *
 * Capability identity (invitation hashes, lease tokens, Ed25519 possession
 * proofs) is deliberately separate from model provider credentials. Nothing in
 * this module reads, writes, or requires any provider secret.
 */
import {
	createHash,
	createPrivateKey,
	createPublicKey,
	sign as edSign,
	verify as edVerify,
	generateKeyPairSync,
	randomBytes,
} from "node:crypto";

const ED25519_PUBLIC_KEY_PREFIX = "ed25519-pem:";
const ED25519_SIGNATURE_PREFIX = "sig:";

export interface Ed25519Keypair {
	/** PEM (SPKI) form of the public key. */
	publicKey: string;
	/** PEM (PKCS#8) form of the private key. */
	privateKey: string;
}

function toBase64Url(input: Buffer): string {
	return input.toString("base64url");
}

function fromBase64Url(input: string): Buffer {
	const buffer = Buffer.from(input, "base64url");
	if (buffer.toString("base64url") !== input) throw new Error("invalid base64url payload");
	return buffer;
}

/** 256-bit opaque token; returns plaintext plus its persisted SHA-256 form. */
export function generateOpaqueToken(): { plaintext: string; hash: string } {
	const plaintext = toBase64Url(randomBytes(32));
	return { plaintext, hash: sha256Hex(plaintext) };
}

export function generateCapabilityKeypair(): Ed25519Keypair {
	const { publicKey, privateKey } = generateKeyPairSync("ed25519");
	return {
		publicKey: `${ED25519_PUBLIC_KEY_PREFIX}${publicKey.export({ type: "spki", format: "pem" }).toString()}`,
		privateKey: privateKey.export({ type: "pkcs8", format: "pem" }).toString(),
	};
}

export function publicKeyHash(publicKey: string): string {
	return sha256Hex(publicKey);
}

export function sha256Hex(input: string): string {
	return createHash("sha256").update(input, "utf8").digest("hex");
}

function rawPublicKey(publicKey: string): string {
	if (!publicKey.startsWith(ED25519_PUBLIC_KEY_PREFIX)) throw new Error("not a remote-child public key");
	return publicKey.slice(ED25519_PUBLIC_KEY_PREFIX.length);
}

export function signPossession(privateKey: string, publicKey: string, nonce: string): string {
	const payload = Buffer.from(`remote-child-possession\n${publicKey}\n${nonce}`, "utf8");
	const key = createPrivateKey(privateKey);
	return `${ED25519_SIGNATURE_PREFIX}${toBase64Url(edSign(null, payload, key))}`;
}

export function verifyPossession(publicKey: string, nonce: string, signature: string): boolean {
	if (!publicKey.startsWith(ED25519_PUBLIC_KEY_PREFIX) || !signature.startsWith(ED25519_SIGNATURE_PREFIX)) {
		return false;
	}
	const payload = Buffer.from(`remote-child-possession\n${publicKey}\n${nonce}`, "utf8");
	try {
		const key = createPublicKey(rawPublicKey(publicKey));
		return edVerify(null, payload, key, fromBase64Url(signature.slice(ED25519_SIGNATURE_PREFIX.length)));
	} catch {
		return false;
	}
}

/** Constant-time-ish comparison for persisted-hash matching (best-effort). */
export function constantTimeEqual(left: string, right: string): boolean {
	const leftBuffer = Buffer.from(left, "utf8");
	const rightBuffer = Buffer.from(right, "utf8");
	if (leftBuffer.length !== rightBuffer.length) return false;
	return createHash("sha256").update(leftBuffer).digest().equals(createHash("sha256").update(rightBuffer).digest());
}
