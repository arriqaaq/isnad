declare module 'sigma/rendering' {
	export type NodeLabelDrawingFunction = (context: CanvasRenderingContext2D, data: Record<string, any>, settings: Record<string, any>) => void;
	export type NodeHoverDrawingFunction = (context: CanvasRenderingContext2D, data: Record<string, any>, settings: Record<string, any>) => void;
}

declare module 'cytoscape-dagre' {
	const ext: any;
	export default ext;
}
