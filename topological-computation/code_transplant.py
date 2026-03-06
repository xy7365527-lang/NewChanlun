"""Code transplant: extract AST subtree from source, adapt, inject into target.

Transplant = topological fold in code space:
source pattern and target need are identified as "same structure",
source AST is grafted onto target AST.

Pure Python, only stdlib (ast). Requires Python 3.10+ for ast.unparse.
"""

from __future__ import annotations

import ast
import copy


# ---------------------------------------------------------------------------
# Extract
# ---------------------------------------------------------------------------

def extract_function_ast(filepath: str, func_name: str) -> ast.FunctionDef:
    """Extract a function's AST node from file.

    Searches module-level and class-level definitions.
    Raises ValueError if function not found.
    """
    with open(filepath, "r", encoding="utf-8") as f:
        source = f.read()

    tree = ast.parse(source, filename=filepath)

    for node in ast.walk(tree):
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
            if node.name == func_name:
                return copy.deepcopy(node)

    raise ValueError(f"Function '{func_name}' not found in {filepath}")


# ---------------------------------------------------------------------------
# Import management
# ---------------------------------------------------------------------------

def add_import(tree: ast.Module, module: str, name: str | None = None) -> ast.Module:
    """Add import statement if not already present.

    If name is None: adds `import module`
    If name is given: adds `from module import name`

    Returns modified tree (mutates in place for efficiency, caller should
    treat original as consumed).
    """
    # Check if import already exists
    for node in ast.walk(tree):
        if name is None and isinstance(node, ast.Import):
            for alias in node.names:
                if alias.name == module:
                    return tree
        elif name is not None and isinstance(node, ast.ImportFrom):
            if node.module == module:
                for alias in node.names:
                    if alias.name == name:
                        return tree

    # Find insertion point: after last import, before first non-import
    insert_idx = 0
    for i, node in enumerate(tree.body):
        if isinstance(node, (ast.Import, ast.ImportFrom)):
            insert_idx = i + 1
        elif isinstance(node, ast.Expr) and isinstance(
            getattr(node, 'value', None), ast.Constant
        ) and isinstance(node.value.value, str):
            # Skip module docstring
            insert_idx = i + 1
        elif insert_idx > 0:
            break

    if name is None:
        new_import = ast.Import(names=[ast.alias(name=module)])
    else:
        new_import = ast.ImportFrom(
            module=module,
            names=[ast.alias(name=name)],
            level=0,
        )

    tree.body.insert(insert_idx, new_import)
    ast.fix_missing_locations(tree)
    return tree


# ---------------------------------------------------------------------------
# Transplant
# ---------------------------------------------------------------------------

def transplant_function(
    source_file: str,
    source_func: str,
    target_file: str,
    insert_after: str | None = None,
    rename: str | None = None,
    param_mapping: dict[str, str] | None = None,
) -> str:
    """Transplant a function from source to target file.

    1. Extract function AST from source
    2. Rename if needed
    3. Remap parameters if needed
    4. Add missing imports (heuristic: scan source file imports)
    5. Insert into target AST
    6. ast.unparse -> return modified target source

    Returns the modified target source code as a string.
    Does NOT write to disk — caller decides whether to write.
    """
    # Step 1: Extract
    func_node = extract_function_ast(source_file, source_func)

    # Step 2: Rename
    if rename:
        func_node.name = rename

    # Step 3: Remap parameters
    if param_mapping:
        func_node = _remap_params(func_node, param_mapping)

    # Step 4: Read target
    with open(target_file, "r", encoding="utf-8") as f:
        target_source = f.read()

    target_tree = ast.parse(target_source, filename=target_file)

    # Step 4b: Add missing imports from source file
    target_tree = _transplant_imports(source_file, target_tree)

    # Step 5: Insert into target
    if insert_after:
        insert_idx = _find_insert_index(target_tree, insert_after)
    else:
        insert_idx = len(target_tree.body)

    target_tree.body.insert(insert_idx, func_node)
    ast.fix_missing_locations(target_tree)

    # Step 6: Unparse
    return ast.unparse(target_tree)


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

class _NameRemapper(ast.NodeTransformer):
    """Rename all occurrences of old names to new names in an AST."""

    def __init__(self, mapping: dict[str, str]) -> None:
        self.mapping = mapping

    def visit_Name(self, node: ast.Name) -> ast.Name:
        if node.id in self.mapping:
            return ast.Name(id=self.mapping[node.id], ctx=node.ctx)
        return node

    def visit_arg(self, node: ast.arg) -> ast.arg:
        if node.arg in self.mapping:
            return ast.arg(arg=self.mapping[node.arg], annotation=node.annotation)
        return node


def _remap_params(
    func_node: ast.FunctionDef,
    param_mapping: dict[str, str],
) -> ast.FunctionDef:
    """Remap parameter names in a function AST node."""
    remapper = _NameRemapper(param_mapping)
    return remapper.visit(func_node)


def _find_insert_index(tree: ast.Module, after_name: str) -> int:
    """Find the index in tree.body to insert after a named definition."""
    for i, node in enumerate(tree.body):
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)):
            if node.name == after_name:
                return i + 1
    # If not found, append at end
    return len(tree.body)


def _transplant_imports(source_file: str, target_tree: ast.Module) -> ast.Module:
    """Add imports from source file that are missing in target.

    Heuristic: copies all imports from source into target if not already present.
    """
    with open(source_file, "r", encoding="utf-8") as f:
        source_code = f.read()

    source_tree = ast.parse(source_code)

    for node in source_tree.body:
        if isinstance(node, ast.Import):
            for alias in node.names:
                target_tree = add_import(target_tree, alias.name)
        elif isinstance(node, ast.ImportFrom):
            module = node.module or ""
            for alias in node.names:
                target_tree = add_import(target_tree, module, alias.name)

    return target_tree
