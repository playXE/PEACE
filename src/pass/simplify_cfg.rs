use super::*;
use crate::ir::*;
use crate::util::{multimap::*, tree::*};
use hashlink::*;
use std::collections::{BTreeSet, HashSet};
pub struct SimplifyCFGPass;

impl FunctionPass for SimplifyCFGPass {
    type Output = ();
    type Err = ();
    fn run(&mut self, f: &mut LIRFunction) -> Result<Self::Output, Self::Err> {
        let code = &mut f.code;
        let n_basic_blocks = code.len();
        let mut out_edges: Vec<HashSet<usize>> = vec![HashSet::new(); n_basic_blocks];
        let mut in_edges: Vec<HashSet<usize>> = vec![HashSet::new(); n_basic_blocks];
        for i in 0..n_basic_blocks as usize {
            let [a, b] = code[i as usize].branch_targets();
            let a = a.map(|x| x.block_id()).flatten();
            let b = b.map(|x| x.block_id()).flatten();
            if let Some(v) = a {
                out_edges[i as usize].insert(v);
                in_edges[v as usize].insert(i);
            }
            if let Some(v) = b {
                out_edges[i as usize].insert(v);
                in_edges[v as usize].insert(i);
            }
        }

        for i in 0..n_basic_blocks as usize {
            if out_edges[i as usize].len() == 1 {
                let j = *out_edges[i as usize].iter().nth(0).unwrap() as usize;
                if in_edges[j as usize].len() == 1 {
                    if *in_edges[j as usize].iter().nth(0).unwrap() == i {
                        out_edges.swap(i as usize, j as usize);
                        out_edges[j as usize].clear();
                        in_edges[j as usize].clear();
                        let v = ::std::mem::replace(
                            &mut code[j as usize],
                            BasicBlock::new(std::usize::MAX, vec![]),
                        );
                        code[i as usize].join(v);
                    }
                }
            }
        }

        let mut dfs_stack: Vec<usize> = Vec::new();
        let mut dfs_visited: Vec<bool> = vec![false; n_basic_blocks];

        dfs_visited[0] = true;
        dfs_stack.push(0);

        while !dfs_stack.is_empty() {
            let current = dfs_stack.pop().unwrap();

            for other in &out_edges[current] {
                if !dfs_visited[*other as usize] {
                    dfs_visited[*other as usize] = true;
                    dfs_stack.push(*other as usize);
                }
            }
        }

        // collect unused blocks
        {
            let unused_blocks: BTreeSet<usize> =
                (0..code.len()).filter(|i| !dfs_visited[*i]).collect();
            let mut tail = n_basic_blocks - 1;
            let mut remap_list: Vec<(usize, usize)> = Vec::new(); // (to, from)
            for id in &unused_blocks {
                while tail > *id {
                    if unused_blocks.contains(&tail) {
                        tail -= 1;
                    } else {
                        break;
                    }
                }

                // Implies tail > 0
                if tail <= *id {
                    break;
                }

                // Now `id` is the first unused block and `tail`
                // is the last used block
                // Let's exchange them
                remap_list.push((*id, tail));
                code.swap(*id, tail);
                tail -= 1;
            }
            while code.len() > tail + 1 {
                code.pop().unwrap();
            }
            for (to, from) in remap_list {
                for bb in code.iter_mut() {
                    let replaced = bb.try_replace_branch_targets(
                        Box::new(Node::Operand(Operand::Block(from))),
                        Box::new(Node::Operand(Operand::Block(to))),
                    );
                    if replaced {}
                }
            }
            // change basic blocks indexes to new one
            for (i, bb) in code.iter_mut().enumerate() {
                bb.id = i;
            }
            //n_basic_blocks = f.basic_blocks.len();
        }
        Ok(())
    }
}
