import type { NarratorGraph } from './types';

export type LayoutMode = 'hierarchical' | 'force';

/**
 * Arrange nodes in a radial layout: center in the middle,
 * teachers in an arc above, students in an arc below.
 * This prevents the cramped single-row problem and makes
 * edges clearly visible as they fan out from the center.
 */
export function applyHierarchicalLayout(graph: NarratorGraph): void {
  const teachers: string[] = [];
  const students: string[] = [];
  let centerNode: string | null = null;

  graph.forEachNode((node, attrs) => {
    if (attrs.narratorType === 'center') centerNode = node;
    else if (attrs.narratorType === 'teacher') teachers.push(node);
    else if (attrs.narratorType === 'student') students.push(node);
  });

  // Center node at origin
  if (centerNode) {
    graph.setNodeAttribute(centerNode, 'x', 0);
    graph.setNodeAttribute(centerNode, 'y', 0);
    graph.setNodeAttribute(centerNode, 'size', 15);
  }

  // Place teachers in an arc above the center (semicircle from left to right)
  const teacherRadius = 200 + teachers.length * 12;
  placeInArc(graph, teachers, teacherRadius, Math.PI, 2 * Math.PI); // top semicircle

  // Place students in an arc below the center
  const studentRadius = 200 + students.length * 12;
  placeInArc(graph, students, studentRadius, 0, Math.PI); // bottom semicircle
}

/**
 * Place nodes along an arc defined by startAngle..endAngle at the given radius.
 * Angles in radians: 0 = right, PI/2 = bottom, PI = left, 3PI/2 = top.
 */
function placeInArc(
  graph: NarratorGraph,
  nodes: string[],
  radius: number,
  startAngle: number,
  endAngle: number,
) {
  if (nodes.length === 0) return;

  const count = nodes.length;
  // Add padding so nodes don't sit at the exact endpoints
  const padding = (endAngle - startAngle) * 0.05;
  const arcStart = startAngle + padding;
  const arcEnd = endAngle - padding;
  const step = count === 1 ? 0 : (arcEnd - arcStart) / (count - 1);

  nodes.forEach((node, i) => {
    const angle = count === 1 ? (arcStart + arcEnd) / 2 : arcStart + i * step;
    const x = radius * Math.cos(angle);
    const y = radius * Math.sin(angle);
    graph.setNodeAttribute(node, 'x', x);
    graph.setNodeAttribute(node, 'y', y);
    graph.setNodeAttribute(node, 'size', 8);
  });
}

/**
 * Create an FA2 layout supervisor running in a Web Worker.
 * Caller is responsible for start/stop/kill lifecycle.
 */
export async function createFA2Supervisor(graph: NarratorGraph) {
  const { default: FA2Layout } = await import('graphology-layout-forceatlas2/worker');
  const { inferSettings } = await import('graphology-layout-forceatlas2');

  const sensibleSettings = inferSettings(graph);
  const supervisor = new FA2Layout(graph, {
    settings: {
      ...sensibleSettings,
      scalingRatio: 10,
      gravity: 1,
      slowDown: 10,
      edgeWeightInfluence: 1,
      barnesHutOptimize: graph.order > 50,
    },
  });

  return supervisor;
}
