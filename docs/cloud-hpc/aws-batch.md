# AWS Batch

Register `cloud/aws/batch-job-definition.yaml` after substituting `KAIRO_IMAGE`.

Required permissions:

- `batch:RegisterJobDefinition`
- `batch:SubmitJob`
- `batch:DescribeJobs`
- Write access to the configured S3 output prefix.

Submit a run:

```bash
AWS_BATCH_JOB_QUEUE=kairo KAIRO_SCENARIO=s3://bucket/scenario.yaml KAIRO_OUTPUT_URI=s3://bucket/output cloud/aws/submit-experiment.sh run-001
```
