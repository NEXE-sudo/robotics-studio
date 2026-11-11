import assert from "node:assert/strict";
import test from "node:test";
import { groupRosGraphByNode, rosNodeId } from "../src/rosGraph.ts";

const endpoint = (nodeName, nodeNamespace, topicType = "std_msgs/msg/String") => ({
  nodeName,
  nodeNamespace,
  topicType,
  reliability: "RELIABLE",
  durability: "VOLATILE",
  history: "KEEP_LAST",
  depth: 10,
});

test("groups graph endpoints by fully qualified node identity", () => {
  const graph = {
    nodes: [
      { name: "worker", namespace: "/" },
      { name: "worker", namespace: "/robot" },
    ],
    topics: [
      {
        name: "/status",
        types: ["std_msgs/msg/String"],
        publishers: [endpoint("worker", "/")],
        subscribers: [endpoint("worker", "/robot")],
      },
    ],
    services: [
      {
        name: "/reset",
        types: ["std_srvs/srv/Empty"],
        serverNodes: ["/worker"],
        clientNodes: ["/robot/worker"],
      },
    ],
    actions: [
      {
        name: "/navigate",
        types: ["nav2_msgs/action/NavigateToPose"],
        serverNodes: ["/robot/worker"],
        clientNodes: ["/worker"],
      },
    ],
    truncated: false,
  };

  const grouped = groupRosGraphByNode(graph);
  const rootNode = grouped.get(rosNodeId("worker", "/"));
  const namespacedNode = grouped.get(rosNodeId("worker", "/robot"));

  assert.equal(rootNode.publishers[0].topicName, "/status");
  assert.deepEqual(rootNode.services.map(({ name }) => name), ["/reset"]);
  assert.deepEqual(rootNode.actionClients.map(({ name }) => name), ["/navigate"]);
  assert.equal(namespacedNode.subscribers[0].topicType, "std_msgs/msg/String");
  assert.deepEqual(namespacedNode.serviceClients.map(({ name }) => name), ["/reset"]);
  assert.deepEqual(namespacedNode.actions.map(({ name }) => name), ["/navigate"]);
});

test("ignores endpoint roles for nodes omitted from a bounded snapshot", () => {
  const graph = {
    nodes: [],
    topics: [{
      name: "/orphan",
      types: ["std_msgs/msg/String"],
      publishers: [endpoint("missing", "/")],
      subscribers: [],
    }],
    services: [{
      name: "/orphan_service",
      types: [],
      serverNodes: ["/missing"],
      clientNodes: [],
    }],
    actions: [],
    truncated: true,
  };

  assert.equal(groupRosGraphByNode(graph).size, 0);
});
