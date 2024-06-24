class TreePosition {
  level: number;
  index: number;
  
  constructor(level: number, index: number) {
    this.level = level;
    this.index = index;
  }
  isLeaf(): boolean {
    return this.level === 0;
  }
  getLeftChild(): TreePosition {
    return new TreePosition(this.level - 1, this.index * 2);
  }
  getRightChild(): TreePosition {
    return new TreePosition(this.level - 1, this.index * 2 + 1);
  }
  getParent(): TreePosition {
    return new TreePosition(this.level + 1, this.index >> 1);
  }
  getSpan(): number {
    return 1 << this.level;
  }
  isNull(): boolean {
    return this.level === 0xffff;
  }
  static newNull(): TreePosition {
    return new TreePosition(0xffff, 0);
  }
}
interface IBinaryTreeJob {
  position: TreePosition;
  left_job: TreePosition;
  right_job: TreePosition;
}


function genLeavesForBinaryTree(numLeaves: number): IBinaryTreeJob[] {
  const leaves: IBinaryTreeJob[] = [];
  for (let i = 0; i < numLeaves; i++) {
    leaves.push({
      position: new TreePosition(0, i),
      left_job: TreePosition.newNull(),
      right_job: TreePosition.newNull(),
    });
  }
  return leaves;
}

/*

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BinaryTreePlanner {
    pub levels: Vec<Vec<BinaryTreeJob>>,
    pub num_leaves: usize,
}
impl BinaryTreePlanner {
    pub fn new(num_leaves: usize) -> Self {
        let mut current = gen_leaves_binary_tree_planner(num_leaves);
        let mut level_index = 1u64;
        let mut levels: Vec<Vec<BinaryTreeJob>> = Vec::new();
        while current.len() > 1 {
            let mut next_level: Vec<BinaryTreeJob> = Vec::new();
            for i in 0..(current.len() / 2) {
                next_level.push(BinaryTreeJob {
                    position: TreePosition::new(level_index, i as u64),
                    left_job: current[i * 2].position,
                    right_job: current[i * 2 + 1].position,
                });
            }
            let mut n_current = next_level.clone();
            levels.push(next_level);

            if current.len() % 2 == 1 {
                n_current.push(current[current.len() - 1]);
            }
            current = n_current;
            level_index += 1;
        }

        Self { levels, num_leaves }
    }
    pub fn get_graphviz(&self) -> String {
        let mut output = String::new();
        output.push_str("digraph G {\n");
        for level in self.levels.iter() {
            for job in level.iter() {
                output.push_str(&format!(
                    "\"{}:{}\" -> \"{}:{}\";\n",
                    job.position.level, job.position.index, job.left_job.level, job.left_job.index
                ));
                output.push_str(&format!(
                    "\"{}:{}\" -> \"{}:{}\";\n",
                    job.position.level,
                    job.position.index,
                    job.right_job.level,
                    job.right_job.index
                ));
            }
        }
        output.push_str("}\n");
        output
    }
}

*/

interface IBinaryTreePlanner {
  levels: IBinaryTreeJob[][];
  num_leaves: number;
}
function createBinaryTreePlanner(numLeaves: number): IBinaryTreePlanner {
  let current = genLeavesForBinaryTree(numLeaves);
  let levelIndex = 1;
  const levels: IBinaryTreeJob[][] = [];
  while (current.length > 1) {
    const nextLevel: IBinaryTreeJob[] = [];
    for (let i = 0, l = Math.floor(current.length / 2); i < l; i++) {
      nextLevel.push({
        position: new TreePosition(levelIndex, i),
        left_job: current[i * 2].position,
        right_job: current[i * 2 + 1].position,
      });
    }
    let nCurrent = nextLevel.slice();
    levels.push(nextLevel);
    if (current.length % 2 === 1) {
      nCurrent.push(current[current.length - 1]);
    }
    current = nCurrent;
    levelIndex += 1;
  }
  return { levels, num_leaves: numLeaves };
}
function getGraphVizForBinaryTreePlanner(planner: IBinaryTreePlanner): string {
  let output = "digraph G {\n";
  for (const level of planner.levels) {
    for (const job of level) {
      output += `"${job.position.level}:${job.position.index}" -> "${job.left_job.level}:${job.left_job.index}";\n`;
      output += `"${job.position.level}:${job.position.index}" -> "${job.right_job.level}:${job.right_job.index}";\n`;
    }
  }
  output += "}\n";
  return output;
}

export type {
  IBinaryTreeJob,
  IBinaryTreePlanner,
};
export {
  TreePosition,
  genLeavesForBinaryTree,
  createBinaryTreePlanner,
  getGraphVizForBinaryTreePlanner,
};
