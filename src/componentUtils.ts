export function canLoadRobotDescription(
  descriptionContent: string | null,
  descriptionPath: string | null,
  workspaceRoot: string | null,
): boolean {
  return Boolean(descriptionContent && descriptionPath && workspaceRoot);
}

export function nodePath(nodeName: string, nodeNamespace: string) {
  const namespace = nodeNamespace === "/" ? "" : nodeNamespace.replace(/\/+$/, "");
  return `${namespace}/${nodeName}`.replace(/\/+/g, "/");
}

export function defaultServiceRequest(serviceType: string) {
  if (
    serviceType === "rcl_interfaces/srv/GetParameters" ||
    serviceType === "rcl_interfaces/srv/GetParameterTypes" ||
    serviceType === "rcl_interfaces/srv/DescribeParameters"
  ) {
    return '{"names":[]}';
  }
  if (serviceType === "rcl_interfaces/srv/ListParameters") {
    return '{"prefixes":[],"depth":0}';
  }
  return "{}";
}

export function shouldResetEventRecoveryAcknowledgement(
  previous: {
    gapDetected: boolean;
    oldestAvailableSequence: number;
    latestSequence: number;
    snapshotIncluded: boolean;
    retainedCriticalEvents: number;
  } | null,
  current: {
    gapDetected: boolean;
    oldestAvailableSequence: number;
    latestSequence: number;
    snapshotIncluded: boolean;
    retainedCriticalEvents: number;
  } | null,
) {
  if (!current?.gapDetected) {
    return Boolean(previous?.gapDetected);
  }
  if (!previous) {
    return true;
  }
  if (!previous.gapDetected) {
    return true;
  }

  return (
    previous.oldestAvailableSequence !== current.oldestAvailableSequence ||
    previous.latestSequence !== current.latestSequence ||
    previous.snapshotIncluded !== current.snapshotIncluded ||
    previous.retainedCriticalEvents !== current.retainedCriticalEvents
  );
}

export function describeEventRecovery(recovery: {
  gapDetected: boolean;
  oldestAvailableSequence: number;
  latestSequence: number;
  snapshotIncluded: boolean;
  retainedCriticalEvents?: number;
} | null) {
  if (!recovery || !recovery.gapDetected) {
    return null;
  }

  const snapshotText = recovery.snapshotIncluded
    ? "Current state snapshots were restored."
    : "A state snapshot is not available yet.";
  const retainedText =
    typeof recovery.retainedCriticalEvents === "number"
      ? ` Retained critical events: ${recovery.retainedCriticalEvents}.`
      : "";

  return `${snapshotText} Oldest retained sequence: ${recovery.oldestAvailableSequence}; latest sequence: ${recovery.latestSequence}.${retainedText}`;
}

export function eventRecoveryActionLabel(recovery: {
  gapDetected: boolean;
  oldestAvailableSequence: number;
  latestSequence: number;
  retainedCriticalEvents?: number;
} | null) {
  if (!recovery || !recovery.gapDetected) {
    return "No recovery action required";
  }
  const retainedText =
    typeof recovery.retainedCriticalEvents === "number"
      ? ` · retained critical events ${recovery.retainedCriticalEvents}`
      : "";
  return `Acknowledge recovery gap · appears from ${recovery.oldestAvailableSequence} to ${recovery.latestSequence}${retainedText}`;
}
