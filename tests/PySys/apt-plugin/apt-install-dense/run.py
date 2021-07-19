import pysys
from pysys.basetest import BaseTest

"""
Validate apt plugin install

Using `rolldice` as a guinea pig: [small and without impacts](https://askubuntu.com/questions/422362/very-small-package-for-apt-get-experimentation)
"""

class AptPlugin(BaseTest):

    def setup(self):
        self.apt_plugin = "/etc/tedge/sm-plugins/apt"
        self.apt_get = "/usr/bin/apt-get"
        self.sudo = "/usr/bin/sudo"


    def plugin_cmd( self, command, outputfile, exit_code, argument=None):
        args=[self.apt_plugin, command]
        if argument:
            args.append(argument)

        process = self.startProcess(
            command=self.sudo,
            arguments=args,
            stdouterr=outputfile,
            expectedExitStatus=f"=={exit_code}",
        )

    def apt_remove(self, package):
            self.startProcess(
            command=self.sudo,
            arguments=[self.apt_get, 'remove', '-y', package],
            abortOnError=False,
        )

class AptPluginInstallTest(AptPlugin):
    def setup(self):
        super().setup()
        self.remove_rolldice_module()
        self.addCleanupFunction(self.remove_rolldice_module)

    def execute(self):
        self.plugin_cmd('list', 'outp_before', 0)
        self.plugin_cmd('install', 'outp_install', 0, "rolldice")
        self.plugin_cmd('list', 'outp_after', 0)

    def validate(self):
        self.assertGrep ("outp_before.out", 'rolldice', contains=False)
        self.assertGrep ("outp_after.out", 'rolldice', contains=True)

    def remove_rolldice_module(self):
        self.apt_remove('rolldice')