import os
import json

def get_repo_info():
    url = os.popen("git config --get remote.origin.url").read().strip()
    hash_val = os.popen("git rev-parse HEAD").read().strip()
