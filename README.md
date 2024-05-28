# Progress
What can already be done:
- [x] Send experiments in a job-array
- [x] Retrieve the results from a given database
What needs to be implemented:
- [x] Detect when a job has failed, and give the option to relaunch them.
	- First filter out all the failed experiments and remake a Job structure for them. (Still per benchmark suite.)
	- Update the experiments.json file to keep track of only the failed ones. But we need somehow to keep track of the busy benchmarks from previous iterations. (Maybe append the new Jobs to the new Job list?)
	- [ ] Main code is there, but there is still testing to be done, needs some patching fixing along the way
- [x] Update the python scripts to read from the new database to put into Pandas

- [x] Add a file to put all the constants (json-filenames for example)
- [ ] Modify experiments.rs to use serde\_json instead of the json package

# Structure
The current structure of benchmarks are:
- Job: which is one (slurm)job per benchmark suite, and it does contain several experiments
	- Experiment (Every rotation of the Sniper arguments has its own experiment, it does contain several benchmarks)
		- Benchmark (Every benchmark has its own parameters to be able to be run.)

# Processing the data
A little python library is added here, where you can process the data into a pandas dataframe. It is not stable, nor well documented but is present if needed.

To install please run `pip3 install /path/to/process_scipts`, and those can be used in your python scripts as:
```python
from process_scripts import process_data
from process_scripts.graph_parameters import *
```
