## Running the site using anaconda environement

Nice tutorial: https://s-canchi.github.io/2021-04-30-jekyll-conda/

First create the new anaconda environmnet using the `environment.yaml`file

```
conda env create -f env.yaml
conda activate docs
```

Once in the environment `docs` install jekyll

```
gem install jekyll bundler
```

Then install necessary jekyll dependencies of the simplefoc docs:

```
# Optionally specify "--path /some/other/dir" to avoid needing root.
bundle install
```

And you're ready to go!

Just make sure that whenever you open your terminal to generate the website to activate the conda environment:
```
conda activate docs
```

## Generating the website

To generate the site locally clone the repo to your local directory and then open terminal inside the repo folder and run:

```
bundle exec jekyll serve
```

Since the site is quiet large sometimes the `--incremental` flag helps with faster execution

```
bundle exec jekyll serve --incremental
```