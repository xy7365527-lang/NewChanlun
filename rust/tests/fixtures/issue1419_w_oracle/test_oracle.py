import copy, unittest
import oracle as o

def event(i,high,low):
 return dict(source_coord=str(i),open=str(low),close=str(high),high=str(high),low=str(low))

class FoldTests(unittest.TestCase):
 def test_unsettled_nontransitive_prefix(self):
  # [0,10], [2,8], [1,11] 相邻均包含，不能伪造方向。
  initial,steps,groups=o.fold([event(0,10,0),event(1,8,2),event(2,11,1)])
  self.assertIsNone(initial);self.assertEqual((steps,groups),([],[]))
 def test_delayed_direction_replays_zero(self):
  initial,steps,groups=o.fold([event(0,10,0),event(1,8,2),event(2,12,4)])
  self.assertEqual(initial['established_at'],'2')
  self.assertEqual(initial['source_coords'],['0','1','2'])
  self.assertEqual([s[0]['action'] for s in steps],['seed','merge','new_group'])
  self.assertEqual(groups[0][0]['members'],['0','1'])
  self.assertEqual((groups[0][0]['high'],groups[0][0]['low']),('10','2'))
  self.assertTrue(groups[0][0]['confirmed']);self.assertFalse(groups[1][0]['confirmed'])
 def test_down_merge(self):
  initial,steps,groups=o.fold([event(0,10,7),event(1,8,5),event(2,9,4)])
  self.assertEqual(initial['direction'],'DOWN')
  self.assertEqual((groups[1][0]['high'],groups[1][0]['low']),('8','4'))
  self.assertEqual(groups[1][0]['high_sources'],['1'])
  self.assertEqual(groups[1][0]['low_sources'],['2'])
  self.assertEqual((groups[1][0]['open'],groups[1][0]['close']),('5','9'))
 def test_equal_extremes_are_sets(self):
  _,steps,groups=o.fold([event(0,3,1),event(1,6,3),event(2,6,4)])
  self.assertEqual(groups[1][0]['high_sources'],['1','2'])
  self.assertEqual(groups[1][0]['low_sources'],['2'])
  self.assertEqual(steps[-1][0]['waiting_reasons'],['equal_extreme_identity'])
  q=o.request(['CC-005.inclusion_group','1'],'id',[],['1','2'])
  self.assertEqual(q['axes']['applicability'],'IN_DOMAIN')
 def test_text_and_array_identity(self):
  c=o.Codec()
  self.assertNotEqual(c.value([1,2]),c.value([2,1]))
  self.assertNotEqual(c.value('1'),c.value(1))
  self.assertNotEqual(c.value({'payload_json':'{"a":1}'},True),c.value({'payload_json':'{"a":1}'},False))
  self.assertTrue(o.diff({'payload_json':'{"a":1}'},{'payload_json':'{"a":1} '}))
 def test_known_protocol_null(self):
  import hashlib
  digest=hashlib.sha256(b'NewChanlun/SContent/1\0'+b'\0'+b'\0'*8).digest()
  self.assertEqual(o.Codec().value(None),digest)

if __name__=='__main__':unittest.main()

