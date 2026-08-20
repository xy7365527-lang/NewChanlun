export interface ConfiguredInstanceRef {
  id: string;
  name: string;
}

export interface ConnectionStateFields {
  wsConnected: boolean;
  httpError: string | null;
  wsError: string | null;
  connectionError: string | null;
}

export type ConnectionStatePatch<T extends ConnectionStateFields> =
  Partial<Omit<T, "connectionError">>;

export interface ConnectionAggregate<T extends ConnectionStateFields> {
  instanceStates: Record<string, T>;
  wsConnected: boolean;
  connectionError: string | null;
}

function combineChannelErrors(
  httpError: string | null,
  wsError: string | null,
): string | null {
  if (httpError && wsError) return `${httpError}；${wsError}`;
  return httpError ?? wsError;
}

export function aggregateConfiguredInstanceState<T extends ConnectionStateFields>(
  instances: readonly ConfiguredInstanceRef[],
  instanceStates: Record<string, T>,
): ConnectionAggregate<T> {
  const errors: string[] = [];
  let wsConnected = false;

  for (const instance of instances) {
    const state = instanceStates[instance.id];
    if (!state) continue;
    wsConnected ||= state.wsConnected;
    if (state.connectionError) {
      errors.push(`[${instance.name}] ${state.connectionError}`);
    }
  }

  return {
    instanceStates,
    wsConnected,
    connectionError: errors.length > 0 ? errors.join("；") : null,
  };
}

export function updateConfiguredInstanceState<T extends ConnectionStateFields>(
  instances: readonly ConfiguredInstanceRef[],
  instanceStates: Record<string, T>,
  id: string,
  defaultState: () => T,
  partial: ConnectionStatePatch<T>,
): ConnectionAggregate<T> {
  if (!instances.some((instance) => instance.id === id)) {
    return aggregateConfiguredInstanceState(instances, instanceStates);
  }

  const merged = {
    ...defaultState(),
    ...instanceStates[id],
    ...partial,
  } as T;
  const next = {
    ...merged,
    connectionError: combineChannelErrors(merged.httpError, merged.wsError),
  } as T;

  return aggregateConfiguredInstanceState(instances, {
    ...instanceStates,
    [id]: next,
  });
}

export function removeConfiguredInstanceState<T extends ConnectionStateFields>(
  instances: readonly ConfiguredInstanceRef[],
  instanceStates: Record<string, T>,
  id: string,
): ConnectionAggregate<T> {
  const remaining = { ...instanceStates };
  delete remaining[id];
  return aggregateConfiguredInstanceState(instances, remaining);
}
