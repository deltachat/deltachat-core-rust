# Concourse CI pipeline

`docs_wheels.yml` is a pipeline for [Concourse CI](https://concourse-ci.org/)
that builds C documentation, Python documentation, Python wheels for `x86_64`
and `aarch64` and Python source packages, and uploads them.

To setup the pipeline run
```
fly -t <your-target> set-pipeline -c docs_wheels.yml -p docs_wheels -l secret.yml
```
where `secret.yml` contains the following secrets:
```
c.delta.chat:
  private_key: |
    -----BEGIN RSA PRIVATE KEY-----
    ...
    -----END RSA PRIVATE KEY-----
devpi:
  login: dc
  password: ...
```
