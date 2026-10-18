"""P0 真控制原语接线。只开放具名 toy，不开放产品及模拟财务成功路径。"""
from dataclasses import asdict
import driver as d
import scheduler as s


class ControlBridge:
    mode = "CONTROL_ONLY"
    execution_authorized = False

    def __init__(self, admission, supervisor, journal, budget, *, seal_directory=None, seal_budget=None):
        d.require(type(admission) is d.ExecutionAdmission and type(budget) is d.ReadBudget,
                  "控制桥缺 P0 admission/共享读取预算")
        d.require(admission.contract.get("scope") == "UNIT_TEST_ONLY", "当前桥只接受具名 toy 合同")
        d.require(admission.contract_sha == journal.header["contract"] == supervisor.contract,
                  "桥接合同摘要不相同")
        d.require(budget.deadline <= admission.business_deadline
                  and journal.header["work_deadline"] <= admission.business_deadline
                  and journal.header["cleanup_deadline"] <= admission.cleanup_deadline
                  and journal.header["seal_deadline"] <= admission.seal_deadline,
                  "桥接延长冻结截止")
        self.admission, self.supervisor, self.journal, self.budget = admission, supervisor, journal, budget
        self.seal_directory, self.seal_budget = seal_directory, seal_budget
        self.seal_attempted = False

    def finish_toy(self, step, owner):
        try:
            key = (step.stage, step.binding.domain, str(step.binding.epoch))
            d.require(self.admission.active_owners.get(key) is owner, "完成对象不是本阶段登记 owner")
            proof = owner.finish()
            entries, _, _, index, pending = self.journal.read()
            d.require(pending == step.sha(), "控制完成没有对应持久消费")
            receipt = s.StageReceipt(index, step.stage, step.scenario, step.sha(),
                                     entries[-1]["predecessor"], d.sha(d.canonical(proof)), "CONTROL_ONLY")
            self.journal.finish(step, receipt)
            return {"receipt": asdict(receipt), "proof": proof, "product_status": "NOT_RUN"}
        except BaseException:
            self.admission.cleanup_all("control-finish-failure")
            raise

    def perform(self, *_):
        raise d.Rejected("产品桥尚缺正式 S/Voice/ARTIFACTS/精确准入；不能返回模拟业务证明")

    def start_toy(self, step, predecessor, output, caps):
        d.require(step.binding.contract_sha == self.admission.contract_sha
                  and step.resources_sha == s.digest(caps)
                  and step.deadline <= self.admission.business_deadline, "控制步骤合同/帽/截止错绑")
        matches = [c for c in self.admission.contract["commands"] if c["stage"] == step.stage]
        d.require(len(matches) == 1 and matches[0]["domain_id"] == step.binding.domain
                  and matches[0]["writer_epoch"] == str(step.binding.epoch)
                  and matches[0]["argv"] == list(step.argv), "控制步骤命令/owner/epoch错绑")
        self.journal.consume(step, predecessor)
        try:
            # 不预调 check_command；OwnedProcess 是一次消费的唯一调用者。
            return d.OwnedProcess(list(step.argv), output, self.supervisor, step.stage,
                                  caps, step.deadline, admission=self.admission)
        except BaseException:
            self.admission.cleanup_all("control-start-failure")
            raise

    def read_domain(self, path):
        return d.read_domain(path, budget=self.budget)

    def custody(self, path):
        d.require(all(o.process.poll() is not None for o in self.admission.active_owners.values()),
                  "托管前 owner 尚未回收")
        return d.custody(path, budget=self.budget)

    def verify_ready(self, *args):
        return d.verify_ready(*args, budget=self.budget)

    def read_original(self, path):
        return d.read_original(path, self.budget)

    def hash_original(self, path):
        return d.hash_original(path, self.budget)

    def active_owner_keys(self):
        return tuple(s.digest(key) for key in self.admission.active_owners)

    def cleanup_all(self, reason):
        keys = self.active_owner_keys()
        rows = self.admission.cleanup_all(reason)
        d.require(len(keys) == len(rows), "P0 回收注册表漏项")
        return tuple(s.CleanupReceipt(key, row["reaped"], d.sha(d.canonical(row)),
                                      "; ".join(row["errors"])) for key, row in zip(keys, rows))

    def seal(self, outcome, receipts, cleanup, deadline):
        d.require(not self.seal_attempted, "封口失败也消费，不覆盖旧原件")
        self.seal_attempted = True
        d.require(type(self.seal_directory) is d.OwnedDirectory and type(self.seal_budget) is d.ReadBudget,
                  "封口必须显式提供已拥有目录及独立冻结预算")
        d.require(deadline == self.seal_budget.deadline <= self.admission.seal_deadline,
                  "封口不得重算/延长截止")
        d.require(outcome in {"CONTROL_COMPLETE", "FAILED", "CANCELLED", "INCOMPLETE"},
                  "控制封口不签发业务 PASS")
        d.require(len(cleanup) == len({c.owner for c in cleanup})
                  and {c.owner for c in cleanup} == set(self.active_owner_keys()), "封口缺完整 owner 回执")
        if not all(c.reaped and not c.error for c in cleanup):
            outcome = "INCOMPLETE"
        value = {"outcome": outcome, "execution_authorized": False, "product_status": "NOT_RUN",
                 "contract_sha": self.admission.contract_sha, "stages": [asdict(r) for r in receipts],
                 "cleanup": [asdict(c) for c in cleanup]}
        try:
            self.seal_budget.check()
            self.seal_budget.bytes_read += len(d.canonical(value)) + 1
            self.seal_budget.check()
            path = self.seal_directory.path / "SEAL.json"
            d.save_new(path, value, deadline=deadline, directory=self.seal_directory)
            raw_sha = d.hash_original(path, self.seal_budget)
            self.seal_directory.verify()
            return s.SealReceipt(raw_sha, outcome, s.digest(value["cleanup"]), s.digest(value["stages"]))
        finally:
            self.seal_directory.close()
