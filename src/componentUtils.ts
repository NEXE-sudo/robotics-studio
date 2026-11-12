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
