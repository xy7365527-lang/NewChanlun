/**
 * Bridges a remote-child admission host into the parent session's
 * `list_subagents()` registry via the provider seam in `rlm-runtime.ts`.
 */
import type { RlmRemoteChildRegistryEntry, RlmRemoteChildRegistryProvider } from "../core/rlm-runtime.js";
import type { RemoteChildAdmissionHost } from "./admission.js";

export function createRemoteChildRegistryProvider(host: RemoteChildAdmissionHost): RlmRemoteChildRegistryProvider {
	return {
		listRemoteChildren(): RlmRemoteChildRegistryEntry[] {
			return host.listEntries();
		},
	};
}
