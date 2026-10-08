//! `anti-slop/no-shape-in-symbol-names`: expectations recorded from the v1 plugin under oxlint 1.86.

use super::{Case, check};

/// tests/no-shape-in-symbol-names.test.ts
#[test]
fn v1() {
    check(
        "anti-slop/no-shape-in-symbol-names",
        "[]",
        &[
            Case { name: "domain name", path: "case.ts", code: "export const invoiceTotal = 1;\n", expected: &[] },
            Case {
                name: "shape in a binding",
                path: "case.ts",
                code: "export const invoiceShape = 1;\n",
                expected: &[(13, 25, "Rename symbol \"invoiceShape\" for its domain role; \"shape\" describes structure rather than ownership.")],
            },
            Case {
                name: "shape is matched case-insensitively",
                path: "case.ts",
                code: "export const SHAPE_VERSION = 1;\n",
                expected: &[(13, 26, "Rename symbol \"SHAPE_VERSION\" for its domain role; \"shape\" describes structure rather than ownership.")],
            },
            Case {
                name: "shape in a function name",
                path: "case.ts",
                code: "export function readShape(): number {\n\treturn 1;\n}\n",
                expected: &[(16, 25, "Rename symbol \"readShape\" for its domain role; \"shape\" describes structure rather than ownership.")],
            },
        ],
    );
}

#[test]
fn identifier_positions() {
    check(
        "anti-slop/no-shape-in-symbol-names",
        "[]",
        &[
            Case {
                name: "references and properties",
                path: "case.ts",
                code: "declare const shape: { shapeKey: number };\nexport const a = shape.shapeKey;\nexport const b = { shape };\nexport const c = { shapeKey: 1, \"shapeString\": 2, [shape.shapeKey]: 3 };\n",
                expected: &[
                    (14, 41, "Rename symbol \"shape\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (23, 31, "Rename symbol \"shapeKey\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (60, 65, "Rename symbol \"shape\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (66, 74, "Rename symbol \"shapeKey\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (95, 100, "Rename symbol \"shape\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (95, 100, "Rename symbol \"shape\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (123, 131, "Rename symbol \"shapeKey\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (155, 160, "Rename symbol \"shape\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (161, 169, "Rename symbol \"shapeKey\" for its domain role; \"shape\" describes structure rather than ownership."),
                ],
            },
            Case {
                name: "patterns",
                path: "case.ts",
                code: "declare const o: { shape: number; other: number };\nconst { shape } = o;\nconst { other: shapeOther = 1 } = o;\nlet shapeA = 0;\n({ shapeA } = { shapeA: 1 });\n({ shapeA = 2 } = {});\n[shapeA] = [1];\nexport { shape, shapeOther };\n",
                expected: &[
                    (19, 24, "Rename symbol \"shape\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (59, 64, "Rename symbol \"shape\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (59, 64, "Rename symbol \"shape\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (87, 97, "Rename symbol \"shapeOther\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (113, 119, "Rename symbol \"shapeA\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (128, 134, "Rename symbol \"shapeA\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (128, 134, "Rename symbol \"shapeA\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (141, 147, "Rename symbol \"shapeA\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (158, 164, "Rename symbol \"shapeA\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (158, 164, "Rename symbol \"shapeA\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (179, 185, "Rename symbol \"shapeA\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (203, 208, "Rename symbol \"shape\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (203, 208, "Rename symbol \"shape\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (210, 220, "Rename symbol \"shapeOther\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (210, 220, "Rename symbol \"shapeOther\" for its domain role; \"shape\" describes structure rather than ownership."),
                ],
            },
            Case {
                name: "modules",
                path: "case.ts",
                code: "import { shape } from \"a\";\nimport { shape as alias, other as shapeOther } from \"b\";\nimport shapeDefault from \"c\";\nimport * as shapeNs from \"d\";\nexport { shape as shapeOut, alias, shapeOther, shapeDefault, shapeNs };\nexport * as shapeAll from \"e\";\nexport { \"shapeString\" as s } from \"f\";\n",
                expected: &[
                    (9, 14, "Rename symbol \"shape\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (9, 14, "Rename symbol \"shape\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (36, 41, "Rename symbol \"shape\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (61, 71, "Rename symbol \"shapeOther\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (91, 103, "Rename symbol \"shapeDefault\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (126, 133, "Rename symbol \"shapeNs\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (153, 158, "Rename symbol \"shape\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (162, 170, "Rename symbol \"shapeOut\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (179, 189, "Rename symbol \"shapeOther\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (179, 189, "Rename symbol \"shapeOther\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (191, 203, "Rename symbol \"shapeDefault\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (191, 203, "Rename symbol \"shapeDefault\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (205, 212, "Rename symbol \"shapeNs\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (205, 212, "Rename symbol \"shapeNs\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (228, 236, "Rename symbol \"shapeAll\" for its domain role; \"shape\" describes structure rather than ownership."),
                ],
            },
            Case {
                name: "classes",
                path: "case.ts",
                code: "export class Shape {\n\t#shape = 1;\n\tshapeField = 2;\n\tget shapeGetter(): number {\n\t\treturn this.#shape;\n\t}\n\thas(o: object): boolean {\n\t\treturn #shape in o;\n\t}\n}\n",
                expected: &[
                    (13, 18, "Rename symbol \"Shape\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (22, 28, "Rename symbol \"shape\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (35, 45, "Rename symbol \"shapeField\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (56, 67, "Rename symbol \"shapeGetter\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (94, 100, "Rename symbol \"shape\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (141, 147, "Rename symbol \"shape\" for its domain role; \"shape\" describes structure rather than ownership."),
                ],
            },
            Case {
                name: "types",
                path: "case.ts",
                code: "interface ShapeLike<TShape> {\n\tshape: TShape;\n\t[shapeKey: string]: unknown;\n\tm(shapeArg: number): void;\n}\ntype ShapeMap = { [KShape in string]: KShape };\ntype Tuple = [shapeLabel: number];\nenum ShapeKind {\n\tShapeA,\n\t\"ShapeB\",\n}\ndeclare namespace ShapeNs {\n\tconst shapeConst: number;\n}\ntype Q = ShapeNs.shapeConst;\nexport type { ShapeLike, ShapeMap, Tuple, Q };\nexport { ShapeKind };\nexport function isShape(v: unknown): v is ShapeLike<number> {\n\treturn v !== null;\n}\nexport function g(this: Window, shapeP: unknown): asserts shapeP {}\n",
                expected: &[
                    (10, 19, "Rename symbol \"ShapeLike\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (20, 26, "Rename symbol \"TShape\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (31, 36, "Rename symbol \"shape\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (38, 44, "Rename symbol \"TShape\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (48, 64, "Rename symbol \"shapeKey\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (79, 95, "Rename symbol \"shapeArg\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (111, 119, "Rename symbol \"ShapeMap\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (125, 131, "Rename symbol \"KShape\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (144, 150, "Rename symbol \"KShape\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (168, 178, "Rename symbol \"shapeLabel\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (194, 203, "Rename symbol \"ShapeKind\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (207, 213, "Rename symbol \"ShapeA\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (246, 253, "Rename symbol \"ShapeNs\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (263, 281, "Rename symbol \"shapeConst\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (294, 301, "Rename symbol \"ShapeNs\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (302, 312, "Rename symbol \"shapeConst\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (328, 337, "Rename symbol \"ShapeLike\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (328, 337, "Rename symbol \"ShapeLike\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (339, 347, "Rename symbol \"ShapeMap\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (339, 347, "Rename symbol \"ShapeMap\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (370, 379, "Rename symbol \"ShapeKind\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (370, 379, "Rename symbol \"ShapeKind\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (399, 406, "Rename symbol \"isShape\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (425, 434, "Rename symbol \"ShapeLike\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (499, 514, "Rename symbol \"shapeP\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (525, 531, "Rename symbol \"shapeP\" for its domain role; \"shape\" describes structure rather than ownership."),
                ],
            },
            Case {
                name: "labels and meta",
                path: "case.ts",
                code: "shapeLabel: for (;;) {\n\tbreak shapeLabel;\n}\nexport const m = import.meta;\n",
                expected: &[
                    (0, 10, "Rename symbol \"shapeLabel\" for its domain role; \"shape\" describes structure rather than ownership."),
                    (30, 40, "Rename symbol \"shapeLabel\" for its domain role; \"shape\" describes structure rather than ownership."),
                ],
            },
            Case {
                name: "strings and comments are not symbols",
                path: "case.ts",
                code: "// shape\nexport const a = \"shape\";\nexport const b = `shape`;\n",
                expected: &[],
            },
        ],
    );
}

#[test]
fn jsx() {
    check(
        "anti-slop/no-shape-in-symbol-names",
        "[]",
        &[Case {
            name: "jsx names",
            path: "case.tsx",
            code: "declare const Shape: (props: { shapeProp: number }) => null;\ndeclare const ui: { Shape: typeof Shape };\nexport const a = <Shape shapeProp={1}></Shape>;\nexport const b = <ui.Shape shapeProp={2} />;\nexport const c = <shape-element />;\nexport const d = <svg:shape xlink:shapeAttr=\"x\" />;\n",
            expected: &[
                (14, 59, "Rename symbol \"Shape\" for its domain role; \"shape\" describes structure rather than ownership."),
                (31, 40, "Rename symbol \"shapeProp\" for its domain role; \"shape\" describes structure rather than ownership."),
                (81, 86, "Rename symbol \"Shape\" for its domain role; \"shape\" describes structure rather than ownership."),
                (95, 100, "Rename symbol \"Shape\" for its domain role; \"shape\" describes structure rather than ownership."),
                (122, 127, "Rename symbol \"Shape\" for its domain role; \"shape\" describes structure rather than ownership."),
                (128, 137, "Rename symbol \"shapeProp\" for its domain role; \"shape\" describes structure rather than ownership."),
                (144, 149, "Rename symbol \"Shape\" for its domain role; \"shape\" describes structure rather than ownership."),
                (173, 178, "Rename symbol \"Shape\" for its domain role; \"shape\" describes structure rather than ownership."),
                (179, 188, "Rename symbol \"shapeProp\" for its domain role; \"shape\" describes structure rather than ownership."),
                (215, 228, "Rename symbol \"shape-element\" for its domain role; \"shape\" describes structure rather than ownership."),
                (255, 260, "Rename symbol \"shape\" for its domain role; \"shape\" describes structure rather than ownership."),
                (267, 276, "Rename symbol \"shapeAttr\" for its domain role; \"shape\" describes structure rather than ownership."),
            ],
        }],
    );
}
