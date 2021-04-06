import pysys
from pysys.basetest import BaseTest

import time

"""
Validate command line option -V
Note: this is a static check and needs to be updated
when a version switch occurs.
"""


class PySysTest(BaseTest):
    def execute(self):
        tedge = "/usr/bin/tedge"

        proc = self.startProcess(
            command=tedge,
            arguments=["-V"],
            stdouterr="tedge",
            expectedExitStatus='==0',
        )

    def validate(self):
        self.assertGrep("tedge.out", "tedge 0.1.0", contains=True)

