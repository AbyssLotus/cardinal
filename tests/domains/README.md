# tests/domains/

Pyramid layer 2 (Vol. V Ch. 8 §8.3): per-domain contract tests. Schemas validate;
composition rules cover every multi-proposer fact type; undeclared reads are test
failures (hidden reads make dependency analysis impossible — Vol. II Ch. 3); ownership
matches Appendix A exactly — a fact type declared in two domains fails here before it
fails anywhere else.
