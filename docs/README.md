## Running the site using anaconda environement

Nice tutorial: https://s-canchi.github.io/2021-04-30-jekyll-conda/

First create the new anaconda environmnet using the `env.yaml`file

```
conda env create -f env.yaml
conda activate mkdocs
```

go to the main directory (no in docs folder) and run the following command:

```
cd ..
```
And you can run the following command to generate the website:

```
mkdocs serve
```

And you're ready to go!

Just make sure that whenever you open your terminal to generate the website to activate the conda environment:
```
conda activate docs
```

## Running with pyhton

If you don't want to use the conda environment, you can also run the code using only python. First install the requirements:

```
pip install -r requirements.txt
```

And then run the following command from the main directory:

```
mkdocs serve
```