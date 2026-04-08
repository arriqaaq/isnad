import type MultiDirectedGraph from 'graphology';
import type { NodeDisplayData, EdgeDisplayData } from 'sigma/types';

export interface NarratorGraphNode extends Partial<NodeDisplayData> {
  narratorId: string;
  narratorType: 'center' | 'teacher' | 'student';
  labelAr: string;
  labelEn: string;
  generation: string | null;
}

export interface NarratorGraphEdge extends Partial<EdgeDisplayData> {
  weight: number;
  curvature?: number;
  parallelIndex?: number;
  parallelMaxIndex?: number;
}

export type NarratorGraph = MultiDirectedGraph<NarratorGraphNode, NarratorGraphEdge>;
