import sys

"""
Validate end to end behaviour for the docker plugin for multiple docker images.

When we install two images
Then these two images are installed
When we install another image and update one of the existing image with newer version
Then there are three images installed, one with newer version
When we delete all the packages
Then docker images are not installed

"""

sys.path.append("software-management-end-to-end")
from environment_sm_management import SoftwareManagement


class SMDockerInstallRemove(SoftwareManagement):
    image1_name="hello-world"
    image2_name="registry"
    image2_version1="2.6.2::docker"
    image2_version2="2.7.1::docker"
    image3_name="docker/getting-started"

    def setup(self):
        super().setup()
       

        if self.dockerplugin != "dockerplugin":
            self.skipTest(
                "Testing the docker plugin is not supported on this platform")

        setup_action = [
            {
                "action": "install",
                "id": self.get_pkgid(self.image1_name),
                "name": self.image1_name,
                "url": " ",
                "version": "::docker",
            },
            {
                "action": "install",
                "id": self.get_pkgid(self.image2_name),
                "name": self.image2_name,
                "url": " ",
                "version": self.image2_version1,
            },
        ]

        self.trigger_action_json(setup_action)
        self.wait_until_succcess()

        self.assertThat("True == value",
                        value=self.check_is_installed(self.image1_name))
        self.assertThat("True == value",
                        value=self.check_is_installed(self.image2_name))
        self.assertThat("False == value",
                        value=self.check_is_installed(self.image3_name))

        self.addCleanupFunction(self.docker_cleanup)

    def execute(self):

        execute_action = [
            {
                "action": "install",
                "id": self.get_pkgid(self.image2_name),
                "name": self.image2_name,
                "url": " ",
                "version": self.image2_version2,
            },
            {
                "action": "install",
                "id": self.get_pkgid(self.image3_name),
                "name": self.image3_name,
                "url": " ",
                "version": "::docker",
            },
        ]

        self.trigger_action_json(execute_action)
        self.wait_until_succcess()

    def validate(self):
        self.assertThat("True == value",
                        value=self.check_is_installed(self.image1_name))
        self.assertThat("True == value",
                        value=self.check_is_installed(self.image2_name))
        self.assertThat("True == value",
                        value=self.check_is_installed(self.image3_name))

    def docker_cleanup(self):
        cleanup_action = [
            {
                "action": "delete",
                "id": self.get_pkgid(self.image2_name),
                "name": self.image2_name,
                "url": " ",
                "version": self.image2_version2,
            },
            {
                "action": "delete",
                "id": self.get_pkgid(self.image3_name),
                "name": self.image3_name,
                "url": " ",
                "version": "::docker",
            },
            {
                "action": "delete",
                "id": self.get_pkgid(self.image1_name),
                "name": self.image1_name,
                "url": " ",
                "version": "::docker",
            },
        ]

        self.trigger_action_json(cleanup_action)
        self.wait_until_succcess()

        self.assertThat("False == value",
                        value=self.check_is_installed(self.image1_name))
        self.assertThat("False == value",
                        value=self.check_is_installed(self.image2_name))
        self.assertThat("False == value",
                        value=self.check_is_installed(self.image3_name))
