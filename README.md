# TitanController
## Progress
This program handles everything with regards to experiments towards an HPC (for now only titan/slurm).
It can
- Create an submit an experiment with varying parameters and benchmarks
- Retrieve all the experiments, and retry them if needed
- List out on the server
    - all the running and past jobs
    - all the docker containers
    - all the traces on titan
- send data to the server
    - new docker containers
    - new traces
- It can delete
    - running jobs
    - docker containers
    - TODO: remove traces

## Features to add
- [ ] Download an individual job to inspect what was wrong.
- [ ] The cache database could become very big very quick. Especially, that it will not handle multi-programs. (It will be double overwrited.) Maybe in the future have one DB per git/bench/trace-hash, and use a lock-file to avoid this behaviour?

## Structure
The structure how the experiment is build works as followed
### Experiment
An Experiment contains a list of BenchmarkSuite.

### BenchmarkSuite
A BenchmarkSuite contains a list of SimulatorParameters.
A BenchmarkSuite assumes that it shares the same folder of benchmarks/traces to be mounted to the VM on the HPC. Which means that this can share the same types of mounts in the same JOB. So there will be only one JOB per BenchmarkSuite, as they all the tasks are sharing the same slurm-script. To differentiate tasks between each other, the JOB is run with a JOB-array, and so every task has its own task id.

### SimulatorParameter
A SimulatorParameter contains a list of BenchmarkParameters.
This is a set of parameters that needs to be passed to the Simulator (for now only Sniper). This is needed to (partially) create the task-script to run the simulator properly.

### BenchmarkParameter
A BenchmarkParameter contains a list of BenchmarkRuns.
This is a set of parameters that is specific for that benchmark. This can include special source-files or make commands, or also some special simulator-parameters specific to that benchmark.

### BenchmarkRun
Finally, this contains just a run index, such that the same benchmark could be run multiple times to eliminate variability if needed.

# Processing the data
A little python library is added here, where you can process the data into a pandas dataframe. It is not stable, nor well documented but is present if needed.

To install please run `pip3 install /path/to/process_scipts`, and those can be used in your python scripts as:
```python
from process_scripts import process_data
from process_scripts.graph_parameters import *
```
## Features to modify
For now all the data is collected into one giant dataframe with one datapoint per row. Ideally this changes to become one set of benchmarkRun (from the hierarchy above), and all the values are in the same row in different columns.
