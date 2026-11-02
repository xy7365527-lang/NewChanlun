import { hostGh } from "./swarm-prompts.mts";
import { prepareSwarmPlan } from "./swarm-planning.mts";

console.log(JSON.stringify(prepareSwarmPlan(hostGh({
  ...process.env, GH_REPO: "xy7365527-lang/NewChanlun", NO_COLOR: "1",
})), null, 2));
