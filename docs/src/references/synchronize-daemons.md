# Introduction

This document briefly explains the problem that exists while
synchronizing the daemons and checking the liveness of the other side
before sending the initial messages. Also, it describes the proposed solutions
and their pros and cons.

# Problem statement

Thin-edge.io has three main issues for synchronizing the daemons

1. The thin-edge daemons depend on another daemon's liveness to delegate or request some work (for example tedge-mapper-c8y depends on tedge-agent to get the software list)
2. The thin-edge daemons are dependent on the liveness of the bridge to communicate with the cloud
3. Some of the daemons are dependent on the `tedge-mapper-c8y`(cloud mapper) file system to create the supported operations file

The thin-edge daemon interdependency was solved using a workaround in the existing solution by creating the persistent session
with the broker when the `daemon is installed`.

# Proposed solutions and their Pros and Cons

## Proposal 1: Use the `health check` mechanism

Use the existing health check message to create the supported operation and send it across to the cloud.
As per this the `daemon` sends the health status message then the `mapper` picks it up and creates the required files and also sends the supported operation for that particular plugin/daemon.
For example, the tedge-agent, when it comes up and sends the health status message the `tedge-agent` supported operations (c8y_Restart, c8y_RemoteAccessConnect, c8y_SoftwareUpdate)  files will be created in `/etc/tedge/operations/c8y` and then sends the supported operations list to the c8y cloud. Now the mapper can send the request to get the software list and forward the response to the c8y cloud.
In the case of other daemons, it's just about creating the supported operations file and updating the operation on behalf of that plugin to the c8y cloud.

Pros: 
-   This will remove the file system dependency when the daemons want to create the supported operation files on the tedge-mapper-c8y  file system.

Cons: 
-   This leads to hard dependency on the list of daemons in the mapper. For example, the mapper has to have a list of `daemons` based on which the correct operation has to be performed.
-   This will be a hard-coded dependency and can’t be updated on the fly as the list of daemons is listed inside the mapper.

## Proposal 2:  Use the `init message` mechanism

Use the new init messages to send the operations to the `cloud` mapper. So, that the mapper can pick this up and create the required files that are mentioned as part of the init message.
Define the structure of the message
The proposed topic will be `tedge/init/<daemon-name>` and the message template could be
{Operation1: <content>, operation2:<>} 
Here some operation files might contain some content and some not.
How to get the file content here, if the operations file has any content, over mqtt?. What is the content format?

Pros:
-   This will remove the file system dependency when the daemons want to create the supported operation files on the tedge-mapper-c8y  file system.
-	No dependency on mapper, no hard-coded list of plugins
-	Very much cloud agnostic

Cons:
-   Need one more topic, and one more message format, which might be difficult to maintain.
