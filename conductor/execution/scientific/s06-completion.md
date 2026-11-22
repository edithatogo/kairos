# S06 Common random numbers

Pair on replicate, full generational entity identity and purpose using the existing versioned ChaCha8 stream derivation. A scenario branch can consume a different entity/purpose stream without shifting a subsequent shared draw. Callers retain the stream across successive draws for a given purpose. The historical scalar sweep seeds preserve their existing mapping.

Use a paired Bernoulli example with thresholds 0.35 and 0.65. The common-uniform difference is Bernoulli(0.30) with variance 0.21. The independent contrast has variance 0.455. Over 60 runs of 1,024 pairs require absolute mean bias at most 0.006, sample variance within 0.015 of each oracle, and at least 50 paired Wilson intervals containing 0.30. Test branch invariance with an extra draw in an unrelated stream and an unpartitioned stream as a negative control. Record the checkpoint's generator, version and distribution provenance.
