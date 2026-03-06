"""Transform code based on topological operations.

fold(funcA, funcB) -> merge two functions, redirect call sites
negate(func) -> mark function as contested (add deprecation comment)
sublate(old, new_code) -> replace function body, keep interface

Pure Python, only stdlib (ast module). Requires Python 3.10+ for ast.unparse.
"""

from __future__ import annotations

import ast
import copy


# ---------------------------------------------------------------------------
# fold: merge two functions
# ---------------------------------------------------------------------------

class _CallRedirector(ast.NodeTransformer):
    """Replace all calls to old_name with calls to new_name."""

    def __init__(self, old_name: str, new_name: str) -> None:
        self.old_name = old_name
        self.new_name = new_name

    def visit_Name(self, node: ast.Name) -> ast.Name:
        if node.id == self.old_name:
            return ast.Name(id=self.new_name, ctx=node.ctx)
        return node

    def visit_Call(self, node: ast.Call) -> ast.Call:
        node = self.generic_visit(node)
        return node


def fold_functions(filepath: str, func_a: str, func_b: str) -> str:
    """Merge func_b into func_a. All calls to func_b become calls to func_a.

    1. Parse AST
    2. Find func_a and func_b nodes
    3. Remove func_b definition
    4. Replace all ast.Call to func_b with calls to func_a
    5. ast.unparse -> return code

    Returns modified source code.
    """
    with open(filepath, "r", encoding="utf-8") as f:
        source = f.read()

    tree = ast.parse(source)

    # Find both functions at module level
    func_a_node = None
    func_b_node = None
    func_b_index = None

    for i, node in enumerate(tree.body):
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
            if node.name == func_a:
                func_a_node = node
            elif node.name == func_b:
                func_b_node = node
                func_b_index = i

    if func_a_node is None:
        raise ValueError(f"Function '{func_a}' not found in {filepath}")
    if func_b_node is None:
        raise ValueError(f"Function '{func_b}' not found in {filepath}")

    # Remove func_b
    tree.body.pop(func_b_index)

    # Redirect all calls from func_b to func_a
    redirector = _CallRedirector(func_b, func_a)
    tree = redirector.visit(tree)
    ast.fix_missing_locations(tree)

    return ast.unparse(tree)


# ---------------------------------------------------------------------------
# negate: mark function as contested
# ---------------------------------------------------------------------------

def negate_function(filepath: str, func_name: str, reason: str) -> str:
    """Mark a function as contested -- add a warning comment at the start of its body.

    Does NOT delete the function. Inserts an Expr(Constant(str)) as the first
    statement to serve as a visible warning, since ast has no comment nodes.

    Returns modified source code.
    """
    with open(filepath, "r", encoding="utf-8") as f:
        source = f.read()

    tree = ast.parse(source)

    target_node = _find_function(tree, func_name)
    if target_node is None:
        raise ValueError(f"Function '{func_name}' not found in {filepath}")

    # Insert warning as first statement (string expression acts as visible marker)
    warning_text = f"CONTESTED: {reason}"
    warning_node = ast.Expr(value=ast.Constant(value=warning_text))

    # Check if first statement is already a contest marker
    if (target_node.body
            and isinstance(target_node.body[0], ast.Expr)
            and isinstance(target_node.body[0].value, ast.Constant)
            and isinstance(target_node.body[0].value.value, str)
            and target_node.body[0].value.value.startswith("CONTESTED:")):
        # Replace existing marker
        target_node.body[0] = warning_node
    else:
        target_node.body.insert(0, warning_node)

    ast.fix_missing_locations(tree)
    return ast.unparse(tree)


# ---------------------------------------------------------------------------
# sublate: replace function body, keep interface
# ---------------------------------------------------------------------------

def sublate_function(filepath: str, old_func: str, new_body: str) -> str:
    """Replace function body while keeping signature.

    The old function is preserved as old_func_superseded.
    The new_body string is parsed as the body of a function with the same signature.

    Returns modified source code.
    """
    with open(filepath, "r", encoding="utf-8") as f:
        source = f.read()

    tree = ast.parse(source)

    target_node = _find_function(tree, old_func)
    if target_node is None:
        raise ValueError(f"Function '{old_func}' not found in {filepath}")

    target_index = None
    for i, node in enumerate(tree.body):
        if node is target_node:
            target_index = i
            break

    if target_index is None:
        # Search inside classes
        for cls_node in tree.body:
            if isinstance(cls_node, ast.ClassDef):
                for j, node in enumerate(cls_node.body):
                    if node is target_node:
                        target_index = j
                        break

    # Create superseded copy
    superseded = copy.deepcopy(target_node)
    superseded.name = f"{old_func}_superseded"

    # Parse new body
    # Wrap in a dummy function to parse
    dummy_source = f"def {old_func}():\n"
    for line in new_body.strip().split("\n"):
        dummy_source += f"    {line}\n"

    dummy_tree = ast.parse(dummy_source)
    new_func_node = dummy_tree.body[0]

    # Keep original signature, replace body
    target_node.body = new_func_node.body

    # Insert superseded version after the original
    # Find in tree.body
    for i, node in enumerate(tree.body):
        if node is target_node:
            tree.body.insert(i + 1, superseded)
            break
    else:
        # Might be inside a class
        for cls_node in tree.body:
            if isinstance(cls_node, ast.ClassDef):
                for j, node in enumerate(cls_node.body):
                    if node is target_node:
                        cls_node.body.insert(j + 1, superseded)
                        break

    ast.fix_missing_locations(tree)
    return ast.unparse(tree)


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _find_function(
    tree: ast.Module,
    func_name: str,
) -> ast.FunctionDef | ast.AsyncFunctionDef | None:
    """Find a function by name at module level or inside classes."""
    for node in ast.walk(tree):
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
            if node.name == func_name:
                return node
    return None
