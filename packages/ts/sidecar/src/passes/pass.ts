import type { Edit } from '#sidecar/syntax/edits';
import type { SourceDocument } from '#sidecar/syntax/source-document';

/** One deterministic formatting rule: reads a document, proposes edits. */
export interface FormattingPass {
	computeEdits(document: SourceDocument): Array<Edit>;
	readonly name: string;
}
