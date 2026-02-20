# CI Configuration

## GitHub Actions Workflow

The CI pipeline is defined in `.github/workflows/ci.yml`.

### What it does

1. Checks out the repository
2. Sets up Python 3.13
3. Installs the package with test dependencies (`pip install -e ".[test]"`)
4. Runs tests with coverage: `pytest -m "not slow" --cov=newchan --cov-report=term-missing`
5. Uploads `coverage.xml` as a build artifact

### Required Secrets

The current CI workflow does **not** require any GitHub Secrets. All dependencies are public PyPI packages.

If future integrations are added (e.g., Codecov, deployment), the following secrets may be needed:

| Secret Name | Purpose | When Needed |
|---|---|---|
| `CODECOV_TOKEN` | Upload coverage to Codecov | If Codecov integration is added |
| `PYPI_API_TOKEN` | Publish to PyPI | If automated releases are added |
| `DATABENTO_API_KEY` | Databento market data | If integration tests hit live API |
| `AV_API_KEY` | Alpha Vantage market data | If integration tests hit live API |
| `IBKR_*` | Interactive Brokers credentials | If IBKR integration tests are added |
| `GOOGLE_GENAI_API_KEY` | Google Gemini API | If Gemini challenger tests hit live API |

### Local Development

```bash
# Install with test dependencies
pip install -e ".[test]"

# Run tests
pytest -m "not slow" --tb=short -q

# Run tests with coverage
pytest --cov=newchan --cov-report=term-missing
```
