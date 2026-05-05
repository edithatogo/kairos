# AWS Batch

Register `cloud/aws/batch-job-definition.yaml` after substituting `KAIRO_IMAGE`.

## Offline validation

Run `python cloud\validate_cloud_hpc.py` from the repository root. The offline validator checks that the AWS templates include the container job definition, Fargate platform capability, resource requirements, array index parameterization, output URI wiring, and checkpoint directory environment variable.

## Live validation

The offline check is not an AWS schema validation. Before marking AWS Batch ready, render the placeholders for a test account, register the job definition, submit a small canary job to a test queue, and record the returned job id plus terminal `SUCCEEDED`/`FAILED` status.

Required permissions:

- `batch:RegisterJobDefinition`
- `batch:SubmitJob`
- `batch:DescribeJobs`
- Write access to the configured S3 output prefix.

Submit a run:

```bash
AWS_BATCH_JOB_QUEUE=kairo KAIRO_SCENARIO=s3://bucket/scenario.yaml KAIRO_OUTPUT_URI=s3://bucket/output cloud/aws/submit-experiment.sh run-001
```
