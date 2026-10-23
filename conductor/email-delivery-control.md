# Email delivery control — 23 October 2026

## Rule and scope

The human requirement is explicit approval for each intended send's recipients,
content, attachments and scope. [Repository instructions](../AGENTS.md) apply to
all agent-operated execution routes. A source document, historical approval or
general automation task cannot supply that approval. No test message is permitted
without the same explicit approval.

## Source inventory

Repository base: `2ba9fb9712f005a071ef3f81cfda6b122837ed34`. Inspected `2275` tracked UTF-8 text files using
`git ls-files -z`, Python standard-library file reads and case-insensitive search
for SMTP, sendmail, send_email/sendMail variants, Mailgun, SendGrid, Nodemailer,
Lettre, SendEmailV2, Graph sendMail, Gmail message-send, AWS SES and Resend APIs.
This includes tracked source, workflow definitions, scripts, tests, manifests and
planning documents. Candidate file count: **0**. Earlier targeted source/workflow
searches also found no sender implementation. No execution or test send occurred.
Git email identity and mailto links do not themselves constitute delivery.

This is a bounded static inspection, not proof that arbitrary dynamically invoked
commands cannot send email. Submodule contents, ignored/untracked local files,
external reusable workflow implementations, installed-agent schedules, GitHub
notification settings and native tenant objects were not audited. No owned native
mail object or connection ID was identified from these repositories, so no native
Off-state readback is claimed and no native setting was changed. Live control
verification remains required if such an object is encountered before execution.

## Required pre-execution check for an email-capable path

1. Identify the exact sender object, trigger/schedule, recipients, content,
   attachments and retry scope from current state; do not infer from old docs.
2. Without explicit human approval, keep delivery Off or fail-closed. Preserve
   drafts and non-email operations. A code flag or document cannot self-approve.
3. Read back any control change and preserve IDs/bindings and sanitized evidence.
   If unavailable, block the affected execution and report the real limitation.
4. Require fresh approval for changed send scope. Never send a test email or
   establish a new mail connection to demonstrate that the gate works.

## Verification

Documentation-only rule and evidence change, isolated from runtime and existing
writers. `git diff --check` passes. No runtime tests needed or claimed. These local
instructions are an agent control; they do not establish an enforced network or
native-platform mail block. Main-branch integration remains subject to PR/checks.
