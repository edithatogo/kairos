import copy
import importlib.util
from pathlib import Path
import tempfile
import unittest
spec=importlib.util.spec_from_file_location('identity',Path(__file__).resolve().parents[1]/'scripts/validate_dependency_identity.py');m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
ROW={'change_type':'added','manifest':'scripts/bootstrap-node-tools/package-lock.json','ecosystem':'npm','name':'http-cache-semantics','version':'0.1.0','package_url':'pkg:npm/http-cache-semantics@0.1.0','license':'BSD-2-Clause','source_repository_url':None,'scope':'runtime','vulnerabilities':[{'severity':'high','advisory_ghsa_id':'GHSA-ch52-4w7c-c8xp'}]}
class Tests(unittest.TestCase):
 def test_exact_alias_and_clean(self):
  self.assertEqual(len(m.validate([[ROW]])['corrected_identity_findings']),1)
  self.assertEqual(m.validate([[]])['rows'],0)
 def test_other_package_path_version_severity_advisory_reject(self):
  for key,value in [('manifest','website/package-lock.json'),('version','4.2.0'),('name','other'),('package_url','pkg:npm/http-cache-semantics'),('source_repository_url','https://github.com/kornelski/http-cache-semantics')]:
   r=copy.deepcopy(ROW);r[key]=value
   with self.subTest(key=key),self.assertRaises(ValueError):m.validate([[r]])
  for key,value in [('severity','critical'),('advisory_ghsa_id','GHSA-new')]:
   r=copy.deepcopy(ROW);r['vulnerabilities'][0][key]=value
   with self.subTest(key=key),self.assertRaises(ValueError):m.validate([[r]])
 def test_graph_missing_and_source_drift_reject(self):
  for bad in [None,[],[{}],[[{'change_type':'added'}]]]:
   with self.assertRaises(ValueError):m.validate(bad)
  with tempfile.TemporaryDirectory() as t:
   with self.assertRaises(OSError):m.validate([[ROW]],Path(t))
 def test_other_high_and_later_page_reject(self):
  r=copy.deepcopy(ROW);r['name']='other'
  with self.assertRaises(ValueError):m.validate([[ROW],[r]])
if __name__=='__main__':unittest.main()
