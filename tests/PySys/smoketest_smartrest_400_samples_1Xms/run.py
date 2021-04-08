import os
import pysys
from pysys.basetest import BaseTest

import sys
sys.path.append('environments')
from environment_c8y import EnvironmentC8y

import time

"""
Roundtrip test C8y 400 samples 1Xms delay

Given a configured system with configured certificate
When we derive from EnvironmentC8y
When we run the smoketest for REST publishing with defaults a size of 400, 10ms delay
Then we validate the data from C8y

X=15ms : Works good
#X=10ms : We observe dataloss ocationally
TODO : Further investigation necessary
"""

class PySysTest(EnvironmentC8y):
    def setup(self):
        super().setup()
        self.log.info("Setup")
        self.addCleanupFunction(self.mycleanup)
        self.timeslot = 15

        # Bad hack to wait until the receive window is empty again.
        # Otherwise, we might download data from a previous test
        time.sleep(self.timeslot)

    def execute(self):
        super().execute()
        self.log.info("Execute")

        script = self.project.tebasedir + "ci/roundtrip_local_to_c8y.py"
        cmd = os.path.expanduser(script)

        sub = self.startPython(
            arguments=[ cmd,
                "-m", "REST",
                "-pub", self.project.exampledir,
                "-u", self.project.username,
                "-t", self.project.tennant,
                "-pass", self.project.c8ypass,
                "-id", self.project.deviceid,
                "-o", "35", # burst should take ?ms
                "-d", "15", # delay in ms
                "-s", "400", # samples
                ],
            stdouterr="stdout",
        )

    def validate(self):
        super().validate()
        self.log.info("Validate")
        self.assertGrep('stdout.out', expr='Data verification PASSED', contains=True)
        self.assertGrep('stdout.out', expr='Timestamp verification PASSED', contains=True)

    def mycleanup(self):
        super().mycleanup()
        self.log.info("MyCleanup")
