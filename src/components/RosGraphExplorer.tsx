import { useState } from "react";
import { groupRosGraphByNode, rosNodeId } from "../rosGraph";
import type {
  RosGraphEndpoint,
  RosGraphNodeDetails,
  RosGraphSnapshot,
} from "../rosGraph";

export type {
  RosGraphAction,
  RosGraphEndpoint,
  RosGraphNode,
  RosGraphService,
  RosGraphSnapshot,
  RosGraphTopic,
} from "../rosGraph";

export interface TopicInspection {
  topicName: string;
  messageType: string;
  jsonPayload: string;
  active: boolean;
  truncated: boolean;
  error: string;
}

export interface SafeServiceInvocation {
  serviceName: string;
  serviceType: string;
  responseJson: string;
  error: string;
  truncated: boolean;
  pending: boolean;
}

export interface SafeActionGoal {
  goalId: string;
  actionName: string;
  actionType: string;
  status: string;
  feedbackJson: string;
  resultJson: string;
  error: string;
  terminal: boolean;
  truncated: boolean;
  cancelPending: boolean;
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

function defaultServiceRequest(serviceType: string) {
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

function SafeServiceForm({
  serviceName,
  serviceType,
  invocation,
  onInvoke,
}: {
  serviceName: string;
  serviceType: string;
  invocation: SafeServiceInvocation | null;
  onInvoke: (serviceName: string, serviceType: string, requestJson: string) => void;
}) {
  const [requestJson, setRequestJson] = useState(() =>
    defaultServiceRequest(serviceType),
  );
  const selected =
    invocation?.serviceName === serviceName &&
    invocation.serviceType === serviceType;

  return (
    <form
      aria-label={`Read-only request for ${serviceName}`}
      onSubmit={(event) => {
        event.preventDefault();
        onInvoke(serviceName, serviceType, requestJson);
      }}
      style={{ display: "grid", gap: 5, marginTop: 6 }}
    >
      <label style={{ opacity: 0.75 }} htmlFor={`request-${serviceName}`}>
        Read-only request · {serviceType}
      </label>
      <textarea
        id={`request-${serviceName}`}
        aria-label={`JSON request for ${serviceName}`}
        value={requestJson}
        onChange={(event) => setRequestJson(event.target.value)}
        rows={3}
        spellCheck={false}
        style={{
          boxSizing: "border-box",
          width: "100%",
          resize: "vertical",
          color: "#ddd",
          background: "#1e1e1e",
          border: "1px solid #444",
          borderRadius: 4,
          padding: 6,
          fontFamily: "monospace",
        }}
      />
      <button
        type="submit"
        disabled={invocation?.pending ?? false}
        style={{ justifySelf: "start" }}
      >
        {selected && invocation.pending ? "Calling…" : "Call read-only service"}
      </button>
      {selected && invocation.error && (
        <div role="alert" style={{ color: "#e99", overflowWrap: "anywhere" }}>
          {invocation.error}
        </div>
      )}
      {selected && invocation.truncated && (
        <div role="status" style={{ color: "#e0b060" }}>
          Response was truncated to fit inspection limits.
        </div>
      )}
      {selected && invocation.responseJson && (
        <pre
          aria-label={`Response from ${serviceName}`}
          style={{
            maxHeight: 220,
            overflow: "auto",
            whiteSpace: "pre-wrap",
            overflowWrap: "anywhere",
            margin: 0,
            padding: 6,
            background: "rgba(0,0,0,0.2)",
          }}
        >
          {invocation.responseJson}
        </pre>
      )}
    </form>
  );
}

function SafeActionForm({
  actionName,
  actionType,
  goal,
  disabled,
  onSend,
  onCancel,
}: {
  actionName: string;
  actionType: string;
  goal: SafeActionGoal | null;
  disabled: boolean;
  onSend: (
    actionName: string,
    actionType: string,
    goalJson: string,
  ) => void;
  onCancel: (goalId: string) => void;
}) {
  const [goalJson, setGoalJson] = useState('{"order":5}');
  const selectedGoal =
    goal?.actionName === actionName && goal.actionType === actionType
      ? goal
      : null;
  return (
    <form
      aria-label={`Safe action goal for ${actionName}`}
      onSubmit={(event) => {
        event.preventDefault();
        onSend(actionName, actionType, goalJson);
      }}
      style={{ display: "grid", gap: 5, marginTop: 6 }}
    >
      <label style={{ opacity: 0.75 }} htmlFor={`goal-${actionName}`}>
        Allowlisted non-motion action · {actionType}
      </label>
      <textarea
        id={`goal-${actionName}`}
        aria-label={`JSON goal for ${actionName}`}
        value={goalJson}
        onChange={(event) => setGoalJson(event.target.value)}
        rows={3}
        spellCheck={false}
        style={{
          boxSizing: "border-box",
          width: "100%",
          resize: "vertical",
          color: "#ddd",
          background: "#1e1e1e",
          border: "1px solid #444",
          borderRadius: 4,
          padding: 6,
          fontFamily: "monospace",
        }}
      />
      <button
        type="submit"
        disabled={disabled || Boolean(goal && !goal.terminal)}
        style={{ justifySelf: "start" }}
      >
        {selectedGoal?.status === "submitting"
          ? "Submitting…"
          : "Send safe action goal"}
      </button>
      {selectedGoal && (
        <div role="status" aria-live="polite">
          Status: {selectedGoal.status}
        </div>
      )}
      {selectedGoal?.error && (
        <div role="alert" style={{ color: "#e99", overflowWrap: "anywhere" }}>
          {selectedGoal.error}
        </div>
      )}
      {selectedGoal?.feedbackJson && (
        <pre
          aria-label={`Feedback from ${actionName}`}
          style={{
            maxHeight: 180,
            overflow: "auto",
            whiteSpace: "pre-wrap",
            overflowWrap: "anywhere",
            margin: 0,
            padding: 6,
            background: "rgba(0,0,0,0.2)",
          }}
        >
          Feedback: {selectedGoal.feedbackJson}
        </pre>
      )}
      {selectedGoal?.resultJson && (
        <pre
          aria-label={`Result from ${actionName}`}
          style={{
            maxHeight: 180,
            overflow: "auto",
            whiteSpace: "pre-wrap",
            overflowWrap: "anywhere",
            margin: 0,
            padding: 6,
            background: "rgba(0,0,0,0.2)",
          }}
        >
          Result: {selectedGoal.resultJson}
        </pre>
      )}
      {selectedGoal?.truncated && (
        <div role="status" style={{ color: "#e0b060" }}>
          Action output was truncated to fit inspection limits.
        </div>
      )}
      {selectedGoal?.goalId && !selectedGoal.terminal && (
        <button
          type="button"
          disabled={selectedGoal.cancelPending}
          onClick={() => onCancel(selectedGoal.goalId)}
          style={{ justifySelf: "start" }}
        >
          {selectedGoal.cancelPending ? "Waiting for cancellation…" : "Cancel goal"}
        </button>
      )}
    </form>
  );
}

export default function RosGraphExplorer({
  graph,
  connected,
  inspection,
  pendingTopic,
  inspectionError,
  safeServiceInvocation,
  actionGoal,
  onStartInspection,
  onStopInspection,
  onInvokeSafeService,
  onSendSafeActionGoal,
  onCancelSafeActionGoal,
}: {
  graph: RosGraphSnapshot | null;
  connected: boolean;
  inspection: TopicInspection | null;
  pendingTopic: string | null;
  inspectionError: string | null;
  safeServiceInvocation: SafeServiceInvocation | null;
  actionGoal: SafeActionGoal | null;
  onStartInspection: (topicName: string) => void;
  onStopInspection: (topicName: string) => void;
  onInvokeSafeService: (
    serviceName: string,
    serviceType: string,
    requestJson: string,
  ) => void;
  onSendSafeActionGoal: (
    actionName: string,
    actionType: string,
    goalJson: string,
  ) => void;
  onCancelSafeActionGoal: (goalId: string) => void;
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

  const nodeDetails = groupRosGraphByNode(graph);
  const nodeTopicList = (
    title: string,
    endpoints: RosGraphNodeDetails["publishers"],
  ) => (
    <div>
      <strong>{title} ({endpoints.length})</strong>
      {endpoints.map((endpoint, index) => (
        <div
          key={`${endpoint.topicName}/${endpoint.topicType}/${index}`}
          style={{ marginTop: 3, paddingLeft: 8, overflowWrap: "anywhere" }}
        >
          <div>{endpoint.topicName}</div>
          <div style={{ opacity: 0.7 }}>
            {endpoint.topicType} · {endpoint.reliability},{" "}
            {endpoint.durability}, {endpoint.history}
            {endpoint.history === "KEEP_LAST" ? ` (${endpoint.depth})` : ""}
          </div>
        </div>
      ))}
    </div>
  );
  const nodeOperationList = (
    title: string,
    operations: RosGraphNodeDetails["services"],
  ) => (
    <div>
      <strong>{title} ({operations.length})</strong>
      {operations.map((operation) => (
        <div key={operation.name} style={{ marginTop: 3, overflowWrap: "anywhere" }}>
          {operation.name}
          <div style={{ opacity: 0.7 }}>{operation.types.join(", ")}</div>
        </div>
      ))}
    </div>
  );

  return (
    <section
      aria-label="Live ROS graph"
      style={{ marginTop: 14, paddingTop: 6, borderTop: "1px solid #333" }}
    >
      <div style={{ fontWeight: 600, marginBottom: 4 }}>Live ROS Graph</div>
      <div style={{ opacity: 0.65, fontSize: 11, marginBottom: 6 }}>
        Read-only discovery with bounded live topic inspection. This panel does
        not invoke arbitrary services or actions. Service calls and action goals
        are restricted to backend allowlists.
      </div>
      {graph.truncated && (
        <div role="status" style={{ color: "#e0b060", marginBottom: 6 }}>
          Graph is large; the displayed snapshot is bounded.
        </div>
      )}
      <GraphSection title="Nodes" count={graph.nodes.length}>
        {graph.nodes.map((node) => {
          const id = rosNodeId(node.name, node.namespace);
          const details = nodeDetails.get(id)!;
          return (
            <details
              key={id}
              style={{
                padding: "5px 7px",
                background: "rgba(255,255,255,0.025)",
                border: "1px solid rgba(255,255,255,0.07)",
                borderRadius: 4,
              }}
            >
              <summary style={{ cursor: "pointer", overflowWrap: "anywhere" }}>
                {nodePath(node.name, node.namespace)} · {details.publishers.length} pub,{" "}
                {details.subscribers.length} sub, {details.services.length} services,{" "}
                {details.actions.length} actions
              </summary>
              <div style={{ padding: "6px 0 2px", display: "grid", gap: 5 }}>
                {nodeTopicList("Publishes", details.publishers)}
                {nodeTopicList("Subscribes", details.subscribers)}
                {nodeOperationList("Service servers", details.services)}
                {nodeOperationList("Service clients", details.serviceClients)}
                {nodeOperationList("Action servers", details.actions)}
                {nodeOperationList("Action clients", details.actionClients)}
              </div>
            </details>
          );
        })}
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
                <button
                  type="button"
                  disabled={!connected || pendingTopic !== null}
                  onClick={() =>
                    inspection?.topicName === topic.name && inspection.active
                      ? onStopInspection(topic.name)
                      : onStartInspection(topic.name)
                  }
                  style={{ justifySelf: "start", marginTop: 4 }}
                >
                  {pendingTopic === topic.name
                    ? "Updating…"
                    : inspection?.topicName === topic.name && inspection.active
                      ? "Stop inspection"
                      : "Inspect messages"}
                </button>
                {inspection?.topicName === topic.name && (
                  <div
                    aria-label={`Inspection for ${topic.name}`}
                    style={{
                      display: "grid",
                      gap: 4,
                      maxWidth: "100%",
                      overflow: "hidden",
                    }}
                  >
                    {inspection.messageType && (
                      <div style={{ opacity: 0.7 }}>
                        {inspection.messageType}
                      </div>
                    )}
                    {inspectionError && (
                      <div role="alert" style={{ color: "#e99" }}>
                        {inspectionError}
                      </div>
                    )}
                    {inspection.error && (
                      <div role="alert" style={{ color: "#e99" }}>
                        {inspection.error}
                      </div>
                    )}
                    {inspection.truncated && (
                      <div role="status" style={{ color: "#e0b060" }}>
                        Message was truncated to fit inspection limits.
                      </div>
                    )}
                    {inspection.jsonPayload && (
                      <pre
                        aria-label="Latest ROS topic message"
                        style={{
                          maxHeight: 220,
                          overflow: "auto",
                          whiteSpace: "pre-wrap",
                          overflowWrap: "anywhere",
                          margin: 0,
                          padding: 6,
                          background: "rgba(0,0,0,0.2)",
                        }}
                      >
                        {inspection.jsonPayload}
                      </pre>
                    )}
                  </div>
                )}
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
            details={
              <>
                {endpointsFor(service.serverNodes, service.clientNodes)}
                {service.safeTypes.map((serviceType) => (
                  <SafeServiceForm
                    key={`${service.name}/${serviceType}`}
                    serviceName={service.name}
                    serviceType={serviceType}
                    invocation={safeServiceInvocation}
                    onInvoke={onInvokeSafeService}
                  />
                ))}
              </>
            }
          />
        ))}
      </GraphSection>
      <GraphSection title="Actions" count={graph.actions.length}>
        {graph.actions.map((action) => (
          <GraphCard
            key={action.name}
            name={action.name}
            types={action.types}
            details={
              <>
                {endpointsFor(action.serverNodes, action.clientNodes)}
                {action.safeTypes.map((actionType) => (
                  <SafeActionForm
                    key={`${action.name}/${actionType}`}
                    actionName={action.name}
                    actionType={actionType}
                    goal={actionGoal}
                    disabled={Boolean(actionGoal && !actionGoal.terminal)}
                    onSend={onSendSafeActionGoal}
                    onCancel={onCancelSafeActionGoal}
                  />
                ))}
              </>
            }
          />
        ))}
      </GraphSection>
    </section>
  );
}
