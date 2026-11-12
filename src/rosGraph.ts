export interface RosGraphEndpoint {
  nodeName: string;
  nodeNamespace: string;
  topicType: string;
  reliability: string;
  durability: string;
  history: string;
  depth: number;
}

export interface RosGraphNode {
  name: string;
  namespace: string;
}

export interface RosGraphTopic {
  name: string;
  types: string[];
  publishers: RosGraphEndpoint[];
  subscribers: RosGraphEndpoint[];
}

export interface RosGraphService {
  name: string;
  types: string[];
  serverNodes: string[];
  clientNodes: string[];
  safeTypes: string[];
}

export interface RosGraphAction {
  name: string;
  types: string[];
  serverNodes: string[];
  clientNodes: string[];
  safeTypes: string[];
}

export interface RosGraphSnapshot {
  nodes: RosGraphNode[];
  topics: RosGraphTopic[];
  services: RosGraphService[];
  actions: RosGraphAction[];
  truncated: boolean;
}

export interface RosGraphNodeTopic extends RosGraphEndpoint {
  topicName: string;
}

export interface RosGraphNodeOperation {
  name: string;
  types: string[];
}

export interface RosGraphNodeDetails {
  publishers: RosGraphNodeTopic[];
  subscribers: RosGraphNodeTopic[];
  services: RosGraphNodeOperation[];
  serviceClients: RosGraphNodeOperation[];
  actions: RosGraphNodeOperation[];
  actionClients: RosGraphNodeOperation[];
}

export function rosNodeId(nodeName: string, nodeNamespace: string): string {
  const namespace =
    nodeNamespace === "/" ? "" : nodeNamespace.replace(/\/+$/, "");
  return `${namespace}/${nodeName}`;
}

export function groupRosGraphByNode(
  graph: RosGraphSnapshot,
): Map<string, RosGraphNodeDetails> {
  const result = new Map<string, RosGraphNodeDetails>();
  for (const node of graph.nodes) {
    result.set(rosNodeId(node.name, node.namespace), {
      publishers: [],
      subscribers: [],
      services: [],
      serviceClients: [],
      actions: [],
      actionClients: [],
    });
  }

  for (const topic of graph.topics) {
    for (const [endpoints, key] of [
      [topic.publishers, "publishers"],
      [topic.subscribers, "subscribers"],
    ] as const) {
      for (const endpoint of endpoints) {
        const node = result.get(
          rosNodeId(endpoint.nodeName, endpoint.nodeNamespace),
        );
        if (node) node[key].push({ ...endpoint, topicName: topic.name });
      }
    }
  }

  for (const service of graph.services) {
    const operation = { name: service.name, types: service.types };
    for (const nodeId of service.serverNodes) {
      result.get(nodeId)?.services.push(operation);
    }
    for (const nodeId of service.clientNodes) {
      result.get(nodeId)?.serviceClients.push(operation);
    }
  }

  for (const action of graph.actions) {
    const operation = { name: action.name, types: action.types };
    for (const nodeId of action.serverNodes) {
      result.get(nodeId)?.actions.push(operation);
    }
    for (const nodeId of action.clientNodes) {
      result.get(nodeId)?.actionClients.push(operation);
    }
  }

  return result;
}
