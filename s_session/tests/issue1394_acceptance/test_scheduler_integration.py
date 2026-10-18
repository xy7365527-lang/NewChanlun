from dataclasses import replace
import unittest
import scheduler as s
import test_scheduler as base


class SchedulerIntegrationTests(unittest.TestCase):
    def test_request_only_database_overlap_between_A_B_rejected(self):
        plan=base.fixture()
        steps=list(plan.steps)
        indices=[i for i,x in enumerate(steps) if x.kind=="business" and x.scenario.startswith("normal/")]
        a,b=indices
        shared="/UNIT_TEST_ONLY/shared-beta.sqlite"
        for i in (a,b):
            requests=list(steps[i].requests)
            requests[1]=replace(requests[1],binding=replace(requests[1].binding,db=shared))
            steps[i]=replace(steps[i],requests=tuple(requests))
        with self.assertRaises(s.Rejected):s.validate_plan(replace(plan,steps=tuple(steps)))

    def test_occurrence_cannot_skip_or_change_original_bytes(self):
        for changed in (False,True):
            plan=base.fixture();steps=list(plan.steps)
            i=next(i for i,x in enumerate(steps) if x.kind=="business")
            request=steps[i].requests[0]
            repeat=replace(request,ordinal=1 if changed else 2)
            if changed:repeat=replace(repeat,binding=replace(repeat.binding,request_sha="a"*64))
            steps[i]=replace(steps[i],requests=steps[i].requests+(repeat,))
            with self.assertRaises(s.Rejected):s.validate_plan(replace(plan,steps=tuple(steps)))
