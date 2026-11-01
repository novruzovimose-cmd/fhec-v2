# FHEC V1 Release Checklist

## Correctness

- [ ] All workspace tests pass
- [ ] Real TFHE boundary tests pass
- [ ] CLI build, analyze and run succeed
- [ ] Invalid programs produce appropriate errors
- [ ] Circuit input and output width reporting verified

## Release engineering

- [ ] Cargo.lock included
- [ ] Windows CI passes
- [ ] Linux CI passes
- [ ] Clean checkout builds
- [ ] README commands verified
- [ ] Supported and unsupported features documented
- [ ] Apache-2.0 LICENSE included
- [ ] Changelog reviewed
- [ ] Release commit tagged

## Security and performance

- [ ] No unsupported security claims
- [ ] Key handling reviewed
- [ ] Hardware calibration limitations documented
- [ ] Default zero-latency predictions explained

Do not publish as a verified V1 until these release gates pass.
