/**
 * useFilteredTopology.ts — Applies filter state to topology data.
 *
 * Returns filtered nodes/links and the max degree for the slider range.
 * Filters: domain, min degree, settlement, instance visibility.
 */

import { useMemo } from "react";
import type { TopologyResponse, TopologyNode } from "../types";
import type { InstanceTraversal } from "../components/TopologyView";
import { classifyNode } from "../utils/classifyNode";
import { useStore, type FilterState } from "./useStore";

interface FilteredResult {
  filteredData: TopologyResponse | null;
  filteredTraversals: InstanceTraversal[];
  maxDegree: number;
}

export function useFilteredTopology(
  data: TopologyResponse | null,
  instanceTraversals: InstanceTraversal[],
): FilteredResult {
  const filters = useStore((s) => s.filters);

  return useMemo(() => {
    if (!data) {
      return { filteredData: null, filteredTraversals: instanceTraversals, maxDegree: 0 };
    }

    const maxDegree = data.nodes.reduce((max, n) => Math.max(max, n.degree), 0);

    // Traversal position node IDs should never be filtered out
    const traversalNodeIds = new Set(instanceTraversals.map((t) => t.position));

    // Build set of accepted node IDs
    const acceptedNodes = new Set<string>();
    for (const node of data.nodes) {
      if (!traversalNodeIds.has(node.id) && !passesFilter(node, filters)) continue;
      acceptedNodes.add(node.id);
    }

    const filteredNodes = data.nodes.filter((n) => acceptedNodes.has(n.id));
    const filteredLinks = data.links.filter((l) => {
      const srcId = typeof l.source === "string" ? l.source : l.source.id;
      const tgtId = typeof l.target === "string" ? l.target : l.target.id;
      return acceptedNodes.has(srcId) && acceptedNodes.has(tgtId);
    });

    const filteredData: TopologyResponse = {
      nodes: filteredNodes,
      links: filteredLinks,
      meta: {
        ...data.meta,
        shown_vertices: filteredNodes.length,
        shown_edges: filteredLinks.length,
      },
    };

    // Filter instance traversals by visible instances
    const filteredTraversals = filterTraversals(instanceTraversals, filters);

    return { filteredData, filteredTraversals, maxDegree };
  }, [data, filters, instanceTraversals]);
}

function passesFilter(node: TopologyNode, filters: FilterState): boolean {
  // Domain filter
  if (filters.enabledDomains.size > 0) {
    const domain = classifyNode(node);
    if (!filters.enabledDomains.has(domain)) return false;
  }

  // Degree threshold
  if (node.degree < filters.minDegree) return false;

  // Settlement filter
  if (filters.settlementFilter === "settled" && !node.settled) return false;
  if (filters.settlementFilter === "unsettled" && node.settled) return false;

  return true;
}

function filterTraversals(
  traversals: InstanceTraversal[],
  filters: FilterState,
): InstanceTraversal[] {
  if (filters.visibleInstances.size === 0) return traversals;
  return traversals.filter((t) => filters.visibleInstances.has(t.instanceId));
}
