from typing import List, Optional
from pathlib import Path
import pandas as pd

from process_scripts.sim_information import SimulatorInformation, SimData

from process_scripts.arguments import SimArgs
from process_scripts.simstdout import SimStdout
from process_scripts.simcpi import SimCpi
from process_scripts.simmcpat import SimMcPat
from process_scripts.simcfg import SimCfg
from process_scripts.simstat import SimStat

import process_scripts.utils as utils


class RunExtractor(SimulatorInformation):
    def __init__(self, dir_path: Path):
        self.dir_path = dir_path
        self.elapsed_time: Optional[float] = None  # to pass to SimMcPat

    def get(self) -> List[SimData]:
        values: List[SimData] = []

        stat_obj = SimStat(self.dir_path)
        stat_obj.create_connection()
        stat_values = stat_obj.get()
        values += stat_values

        self.elapsed_time = self.get_elapsed_time(stat_values)

        sources = [SimCfg, SimStdout, SimCpi]
        for Src in sources:
            obj = Src(self.dir_path)
            values += obj.get()

        mcpat = SimMcPat(self.dir_path)
        mcpat.set_time(self.elapsed_time)  # inject time
        values += mcpat.get()

        return values

    @staticmethod
    def get_elapsed_time(values: List[SimData]) -> Optional[float]:
        for sim_data in values:
            if ("stats", "performance_model", "elapsed_time") == (sim_data.source, sim_data.section, sim_data.key):
                return sim_data.value / 1e15
        return None

class BenchExtractor:
    def __init__(self, dir_path: Path):
        self.dir_path = dir_path

    def get(self, sim_args: SimData):
        data = {}
        for bench in utils.get_all_subdirs(self.dir_path):
            bench_name = bench.stem + ".".join(bench.suffixes)
            bench_data = {}
            for exp_nr in utils.get_all_subdirs(bench):
                print(f"Processing {exp_nr}")
                run_info = RunExtractor(exp_nr)
                run_data = run_info.get()
                run_data += sim_args
                bench_data[exp_nr.stem] = run_data
            data[bench_name] = bench_data

        return data

class SimParamExtractor:
    def __init__(self, dir_path: Path):
        self.dir_path = dir_path

    def get(self):
        all_exp = {}
        for exp in utils.get_all_subdirs(self.dir_path):
            sim_args = SimArgs(exp)
            sim_args_data = sim_args.get()

            bench_extr = BenchExtractor(exp)
            bench_data = bench_extr.get(sim_args_data)

            all_exp[exp.stem] = bench_data
        return all_exp

def sim_data_to_df(nested: dict) -> pd.DataFrame:
    rows = []
    row_index = []
    col_keys = set()

    for param_id, benchmarks in nested.items():
        for benchmark, runs in benchmarks.items():
            for run_idx, simdata_list in runs.items():
                row_data = {}
                row_index.append((param_id, benchmark, run_idx))

                for sim in simdata_list:
                    col = (sim.source, sim.section, sim.key, sim.core)
                    row_data[col] = sim.value
                    col_keys.add(col)

                rows.append(row_data)

    # Build DataFrame
    all_cols = sorted(col_keys)
    df = pd.DataFrame(rows, index=pd.MultiIndex.from_tuples(row_index, names=["param_id", "benchmark", "run_idx"]))
    df = df.reindex(columns=all_cols)  # fill missing columns with NaN
    df.columns = pd.MultiIndex.from_tuples(df.columns, names=["source", "section", "key", "core"])

    return df
