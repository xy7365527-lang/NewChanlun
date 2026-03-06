"""Test script for the Topological Coding Agent.

Runs three tests:
1. agent.analyze() — graph stats, fold candidates, gaps
2. agent.execute_command("python -m pytest test_engine.py -v") — test execution
3. agent.task("add a /status HTTP endpoint to daemon.py") — task search

Outputs results to tmp/coding_agent_test.txt
"""
import json
import sys
import os

# Ensure we can import from the topological-computation directory
script_dir = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, script_dir)

from coding_agent import CodingAgent

def main():
    workspace = script_dir
    output_lines = []

    def log(msg):
        print(msg)
        output_lines.append(msg)

    log("=" * 70)
    log("TOPOLOGICAL CODING AGENT TEST")
    log("=" * 70)

    # Initialize agent
    agent = CodingAgent(workspace)
    log(f"\nWorkspace: {workspace}")
    log(f"Code complex: {len(agent.graph.active_vertex_ids())} vertices, "
        f"{len(agent.graph.active_edges())} edges")

    # --- Test 1: analyze() ---
    log("\n" + "=" * 70)
    log("TEST 1: agent.analyze()")
    log("=" * 70)
    analysis = agent.analyze()
    log(json.dumps(analysis, indent=2))

    # --- Test 2: execute_command (run tests) ---
    log("\n" + "=" * 70)
    log("TEST 2: agent.execute_command('python -m pytest test_engine.py -v')")
    log("=" * 70)
    cmd_result = agent.execute_command("python -m pytest test_engine.py -v")
    log(json.dumps(cmd_result, indent=2))

    # --- Test 3: task() ---
    log("\n" + "=" * 70)
    log("TEST 3: agent.task('add a /status HTTP endpoint to daemon.py')")
    log("=" * 70)
    task_result = agent.task("add a /status HTTP endpoint to daemon.py")
    log(json.dumps(task_result, indent=2))

    # Write output
    output_path = os.path.join(os.path.dirname(script_dir), "tmp", "coding_agent_test.txt")
    os.makedirs(os.path.dirname(output_path), exist_ok=True)
    with open(output_path, "w", encoding="utf-8") as f:
        f.write("\n".join(output_lines))
    log(f"\nResults written to {output_path}")

if __name__ == "__main__":
    main()
