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
}

export interface RosGraphAction {
  name: string;
  types: string[];
  serverNodes: string[];
  clientNodes: string[];
}

export interface RosGraphSnapshot {
  nodes: RosGraphNode[];
  topics: RosGraphTopic[];
  services: RosGraphService[];
  actions: RosGraphAction[];
  truncated: boolean;
}

function nodePath(nodeName: string, nodeNamespace: string) {
  return `${nodeNamespace === "/" ? "" : nodeNamespace}/${nodeName}`;
}

function TypeList({ types }: { types: string[] }) {
  if (types.length === 0) return <span style={{ opacity: 0.6 }}>Unknown type</span>;
  return (
    <div style={{ color: "#b7d7f0", overflowWrap: "anywhere" }}>
      {types.join(", ")}
    </div>
  );
}

function GraphSection({
  title,
  count,
  children,
}: {
  title: string;
  count: number;
  children: React.ReactNode;
}) {
  return (
    <details style={{ marginTop: 8 }} open>
      <summary
        style={{
          cursor: "pointer",
          padding: "6px 0",
          borderTop: "1px solid rgba(255,255,255,0.08)",
          fontWeight: 600,
        }}
      >
        {title} ({count})
      </summary>
      <div style={{ display: "flex", flexDirection: "column", gap: 5 }}>
        {children}
      </div>
    </details>
  );
}

function GraphCard({
  name,
  types,
  details,
}: {
  name: string;
  types: string[];
  details?: React.ReactNode;
}) {
  return (
    <details
      style={{
        padding: "5px 7px",
        background: "rgba(255,255,255,0.025)",
        border: "1px solid rgba(255,255,255,0.07)",
        borderRadius: 4,
      }}
    >
      <summary style={{ cursor: "pointer", overflowWrap: "anywhere" }}>
        {name}
      </summary>
      <div style={{ padding: "6px 0 2px", display: "grid", gap: 5 }}>
        <TypeList types={types} />
        {details}
      </div>
    </details>
  );
}

export default function RosGraphExplorer({
  graph,
  connected,
}: {
  graph: RosGraphSnapshot | null;
  connected: boolean;
}) {
  if (!graph) {
    return (
      <div
        role="status"
        style={{ marginTop: 12, paddingTop: 8, opacity: 0.65 }}
      >
        {connected
          ? "Waiting for ROS graph details..."
          : "ROS graph details are unavailable while disconnected."}
      </div>
    );
  }

  const endpointList = (
    title: string,
    endpoints: RosGraphEndpoint[],
  ) => (
    <div>
      <strong>{title} ({endpoints.length})</strong>
      {endpoints.map((endpoint, index) => (
        <div
          key={`${endpoint.nodeNamespace}/${endpoint.nodeName}/${index}`}
          style={{
            marginTop: 3,
            paddingLeft: 8,
            borderLeft: "1px solid #444",
            overflowWrap: "anywhere",
          }}
        >
          <div>{nodePath(endpoint.nodeName, endpoint.nodeNamespace)}</div>
          <div style={{ opacity: 0.7 }}>{endpoint.topicType}</div>
          <div style={{ opacity: 0.65 }}>
            {endpoint.reliability}, {endpoint.durability}, {endpoint.history}
            {endpoint.history === "KEEP_LAST" ? ` (${endpoint.depth})` : ""}
          </div>
        </div>
      ))}
    </div>
  );

  const endpointsFor = (servers: string[], clients: string[]) => (
    <div style={{ display: "grid", gap: 4, overflowWrap: "anywhere" }}>
      <div>
        <strong>Servers:</strong> {servers.length ? servers.join(", ") : "none"}
      </div>
      <div>
        <strong>Clients:</strong> {clients.length ? clients.join(", ") : "none"}
      </div>
    </div>
  );

  return (
    <section
      aria-label="Live ROS graph"
      style={{ marginTop: 14, paddingTop: 6, borderTop: "1px solid #333" }}
    >
      <div style={{ fontWeight: 600, marginBottom: 4 }}>Live ROS Graph</div>
      <div style={{ opacity: 0.65, fontSize: 11, marginBottom: 6 }}>
        Read-only discovery. This panel does not call services or send action
        goals.
      </div>
      {graph.truncated && (
        <div role="status" style={{ color: "#e0b060", marginBottom: 6 }}>
          Graph is large; the displayed snapshot is bounded.
        </div>
      )}
      <GraphSection title="Nodes" count={graph.nodes.length}>
        {graph.nodes.map((node) => (
          <div
            key={`${node.namespace}/${node.name}`}
            style={{ overflowWrap: "anywhere", fontFamily: "monospace" }}
          >
            {nodePath(node.name, node.namespace)}
          </div>
        ))}
      </GraphSection>
      <GraphSection title="Topics" count={graph.topics.length}>
        {graph.topics.map((topic) => (
          <GraphCard
            key={topic.name}
            name={topic.name}
            types={topic.types}
            details={
              <>
                {endpointList("Publishers", topic.publishers)}
                {endpointList("Subscribers", topic.subscribers)}
              </>
            }
          />
        ))}
      </GraphSection>
      <GraphSection title="Services" count={graph.services.length}>
        {graph.services.map((service) => (
          <GraphCard
            key={service.name}
            name={service.name}
            types={service.types}
            details={endpointsFor(service.serverNodes, service.clientNodes)}
          />
        ))}
      </GraphSection>
      <GraphSection title="Actions" count={graph.actions.length}>
        {graph.actions.map((action) => (
          <GraphCard
            key={action.name}
            name={action.name}
            types={action.types}
            details={endpointsFor(action.serverNodes, action.clientNodes)}
          />
        ))}
      </GraphSection>
    </section>
  );
}
