
# Continuous Integration Scripts for Delta Chat

Continuous Integration is run through CircleCI
but is largely independent of it. 


## Generating docker containers for performing build step work 

All tests, docs and wheel building is run in docker containers:

- **coredeps/Dockerfile** specifies an image that contains all 
  of Delta Chat's core dependencies as linkable libraries. 
  It also serves to run python tests and build wheels 
  (binary packages for Python). 

- **doxygen/Dockerfile** specifies an image that contains
  the doxygen tool which is used to generate C-docs. 

To run tests locally you can pull existing images from "docker.io",
the hub for sharing Docker images::

    docker pull deltachat/coredeps
    docker pull deltachat/doxygen 

or you can build the docker images yourself locally
to avoid the relatively large download:: 
 
    cd ci_scripts  # where all CI things are 
    docker build -t deltachat/coredeps docker-coredeps
    docker build -t deltachat/doxygen docker-doxygen 

## ci_run.sh (main entrypoint called by circle-ci)

Once you have the docker images available 
you can run python testing, documentation generation 
and building binary wheels:: 

    sh DOCS=1 TESTS=1 ci_scripts/ci_run.sh 
    
## ci_upload.sh (uploading artifacts on success) 

- python docs to `https://py.delta.chat/_unofficial_unreleased_docs/<BRANCH>`

- doxygen docs to `https://c.delta.chat/_unofficial_unreleased_docs/<BRANCH>`

- python wheels to `https://m.devpi.net/dc/<BRANCH>`
  so that you install fully self-contained wheels like this:
  `pip install -U -i https://m.devpi.net/dc/<BRANCH> deltachat`



