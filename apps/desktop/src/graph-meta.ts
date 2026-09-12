import type { GraphNode, ShellCopy } from "./protocol";

export const GRAPH_RELATION_META: Record<
  NonNullable<GraphNode["relation"]>,
  { order: number; label: (copy: ShellCopy) => string }
> = {
  upstream: { order: 0, label: (copy) => copy.graphUpstream },
  center: {
    order: 1,
    label: (copy) => copy.graphCenter.replace("：{issue}", "").replace(": {issue}", ""),
  },
  both: { order: 2, label: (copy) => copy.graphBoth },
  downstream: { order: 3, label: (copy) => copy.graphDownstream },
};
