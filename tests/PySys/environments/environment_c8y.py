import json
import base64
from pysys.constants import FAILED
import requests
from pysys.basetest import BaseTest

"""
Environment to manage automated connect and disconnect to c8y

Tests that derive from class EnvironmentC8y use automated connect and
disconnect to Cumulocity. Additional checks are made for the status of
service mosquitto and service tedge-mapper.
"""


class Cumulocity(object):
    """Class to retrieve information about Cumulocity.
    TODO : Review if we download enough data -> pageSize
    """

    c8y_url = ""
    tenant_id = ""
    username = ""
    password = ""
    auth = ""
    device_id = ""
    timeout_req = ""

    def __init__(self, c8y_url, tenant_id, username, password, device_id, log):
        self.c8y_url = c8y_url
        self.tenant_id = tenant_id
        self.username = username
        self.password = password
        self.device_id = device_id
        self.timeout_req = 80  # seconds, got timeout with 60s
        self.operation_id = None
        self.log = log

        self.auth = ('%s/%s' % (self.tenant_id, self.username), self.password)

    def request(self, method, url_path, **kwargs) -> requests.Response:
        return requests.request(method, self.c8y_url + url_path, auth=self.auth, **kwargs)

    def get_all_devices(self) -> requests.Response:
        params = {
            "fragmentType": "c8y_IsDevice"
        }
        res = requests.get(
            url=self.c8y_url + "/inventory/managedObjects", params=params, auth=self.auth)

        return self.to_json_response(res)

    def to_json_response(self, res: requests.Response):
        if res.status_code != 200:
            raise Exception(
                "Received invalid response with exit code: {}, reason: {}".format(res.status_code, res.reason))
        return json.loads(res.text)

    def get_all_devices_by_type(self, type: str) -> requests.Response:
        params = {
            "fragmentType": "c8y_IsDevice",
            "type": type,
            "pageSize": 100,
        }
        res = requests.get(
            url=self.c8y_url + "/inventory/managedObjects", params=params, auth=self.auth)
        return self.to_json_response(res)

    def get_all_thin_edge_devices(self) -> requests.Response:
        return self.get_all_devices_by_type("thin-edge.io")

    def get_thin_edge_device_by_name(self, device_id: str):
        json_response = self.get_all_devices_by_type("thin-edge.io")
        for device in json_response['managedObjects']:
            if device_id in device['name']:
                return device
        return None

    def get_header(self):
        auth = bytes(
            f"{self.tenant_id}/{self.username}:{self.password}", "utf-8")
        header = {
            b"Authorization": b"Basic " + base64.b64encode(auth),
            b"content-type": b"application/json",
            b"Accept": b"application/json",
        }
        return header

    def trigger_log_request(self, log_file_request_payload):
        url = f"https://{self.c8y_url}/devicecontrol/operations"
        log_file_request_payload = {
            "deviceId": self.device_id,
            "description": "Log file request",
            "c8y_LogfileRequest": log_file_request_payload,
        }
        req = requests.post(
            url, json=log_file_request_payload, headers=self.get_header(), timeout=self.timeout_req
        )
        jresponse = json.loads(req.text)

        # self.log.info("Response status: %s", req.status_code)
        # self.log.info("logfile path: %s", json.dumps(jresponse, indent=4))

        self.operation = jresponse
        self.operation_id = jresponse.get("id")

        if not self.operation_id:
            raise SystemError("field id is missing in response")

        # self.log.info("Started operation: %s", self.operation)

        req.raise_for_status()

    def check_if_log_req_complete(self):
        """Check if log received"""
        url = f"https://{self.c8y_url}/devicecontrol/operations/{self.operation_id}"
        req = requests.get(url, headers=self.get_header(),
                           timeout=self.timeout_req)

        req.raise_for_status()

        jresponse = json.loads(req.text)

        ret = ""

        log_response = jresponse.get("c8y_LogfileRequest")
        # check if the response contains the logfile
        log_file = log_response.get("file")
        self.log.info("log response %s", log_file)
        if log_file != None:
            ret = log_file
        return ret


class EnvironmentC8y(BaseTest):
    cumulocity: Cumulocity

    def setup(self):
        self.log.debug("EnvironmentC8y Setup")

        if self.project.c8yurl == "":
            self.abort(
                FAILED, "Cumulocity tenant URL is not set. Set with the env variable C8YURL")
        if self.project.tenant == "":
            self.abort(
                FAILED, "Cumulocity tenant ID is not set. Set with the env variable C8YTENANT")
        if self.project.username == "":
            self.abort(
                FAILED, "Cumulocity tenant username is not set. Set with the env variable C8YUSERNAME")
        if self.project.c8ypass == "":
            self.abort(
                FAILED, "Cumulocity tenant password is not set. Set with the env variable C8YPASS")
        if self.project.deviceid == "":
            self.abort(
                FAILED, "Device ID is not set. Set with the env variable C8YDEVICEID")

        self.tedge = "/usr/bin/tedge"
        self.tedge_mapper_c8y = "tedge-mapper-c8y"
        self.sudo = "/usr/bin/sudo"
        self.systemctl = "/usr/bin/systemctl"
        self.log.info("EnvironmentC8y Setup")
        self.addCleanupFunction(self.myenvcleanup)

        # Check if tedge-mapper is in disabled state
        serv_mapper = self.startProcess(
            command=self.systemctl,
            arguments=["status", self.tedge_mapper_c8y],
            stdouterr="serv_mapper1",
            expectedExitStatus="==3",  # 3: disabled
        )

        # Connect the bridge
        connect = self.startProcess(
            command=self.sudo,
            arguments=[self.tedge, "connect", "c8y"],
            stdouterr="tedge_connect",
        )

        # Test the bridge connection
        connect = self.startProcess(
            command=self.sudo,
            arguments=[self.tedge, "connect", "c8y", "--test"],
            stdouterr="tedge_connect_test",
        )

        # Check if mosquitto is running well
        serv_mosq = self.startProcess(
            command=self.systemctl,
            arguments=["status", "mosquitto"],
            stdouterr="serv_mosq2",
        )

        # Check if tedge-mapper is active again
        serv_mapper = self.startProcess(
            command=self.systemctl,
            arguments=["status", self.tedge_mapper_c8y],
            stdouterr="serv_mapper3",
        )

        self.cumulocity = Cumulocity(
            self.project.c8yurl, self.project.tenant, self.project.username, self.project.c8ypass, self.project.deviceid, self.log)

    def execute(self):
        self.log.debug("EnvironmentC8y Execute")

    def validate(self):
        self.log.debug("EnvironmentC8y Validate")

        # Check if mosquitto is running well
        serv_mosq = self.startProcess(
            command=self.systemctl,
            arguments=["status", "mosquitto"],
            stdouterr="serv_mosq",
        )

        # Check if tedge-mapper is active
        serv_mapper = self.startProcess(
            command=self.systemctl,
            arguments=["status", self.tedge_mapper_c8y],
            stdouterr="serv_mapper4",
        )

    def myenvcleanup(self):
        self.log.debug("EnvironmentC8y Cleanup")

        # Disconnect Bridge
        disconnect = self.startProcess(
            command=self.sudo,
            arguments=[self.tedge, "disconnect", "c8y"],
            stdouterr="tedge_disconnect",
        )

        # Check if tedge-mapper is disabled
        serv_mosq = self.startProcess(
            command=self.systemctl,
            arguments=["status", self.tedge_mapper_c8y],
            stdouterr="serv_mapper5",
            expectedExitStatus="==3",
        )
