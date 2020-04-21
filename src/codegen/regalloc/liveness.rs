use super::*;
use crate::codegen::*;
use crate::ir::Node as IRNode;
use crate::ir::*;
use hashlink::{LinkedHashMap, LinkedHashSet};
use std::fmt;
pub use NodeType::*;
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum NodeType {
    Def,
    Use,
    Copy,
    Machine,
}

/// GraphNode represents a node in the interference graph.
#[derive(Clone, Copy, PartialEq)]
pub struct Node {
    /// temp ID (could be register)
    temp: usize,
    /// assigned color
    color: Option<usize>,
    /// temp register group (which machine register class we should assign)
    group: RegGroup,
    /// cost to spill this temp
    spill_cost: f32,
    /// cost to freeze this temp
    freeze_cost: f32,
}

impl fmt::Debug for Node {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(
            f,
            "Node({}): color={:?}, group={:?}, spill_cost={}",
            self.temp, self.color, self.group, self.spill_cost
        )
    }
}

/// Move represents a move between two nodes (referred by index)
/// We need to know the moves so that we can coalesce.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Move {
    pub from: usize,
    pub to: usize,
}

impl fmt::Debug for Move {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "Move ({} -> {})", self.from, self.to)
    }
}

#[inline(always)]
fn is_precolored(reg: usize) -> bool {
    if reg < MACHINE_ID_END {
        true
    } else {
        false
    }
}

#[inline(always)]
fn is_usable(reg: usize) -> bool {
    if all_usable_regs()
        .iter()
        .any(|x| x.any_reg_id() == get_color_for_precolored(reg))
    {
        true
    } else {
        false
    }
}

#[inline(always)]
/// checks if a reg is machine register. If so, return its color
/// otherwise return the reg
fn c(u: usize) -> usize {
    if is_precolored(u) {
        get_color_for_precolored(u)
    } else {
        u
    }
}

/// InterferenceGraph represents the interference graph, including
/// * the graph
/// * all the nodes and its NodeIndex (a node is referred to by NodeIndex)
/// * all the moves
pub struct InterferenceGraph {
    nodes: LinkedHashMap<usize, Node>,

    adj_set: LinkedHashSet<(usize, usize)>,
    adj_list: LinkedHashMap<usize, LinkedHashSet<usize>>,
    degree: LinkedHashMap<usize, usize>,
    moves: LinkedHashSet<Move>,
}

impl InterferenceGraph {
    /// creates a new graph
    fn new() -> InterferenceGraph {
        InterferenceGraph {
            adj_set: LinkedHashSet::new(),
            adj_list: LinkedHashMap::new(),
            degree: LinkedHashMap::new(),
            nodes: LinkedHashMap::new(),
            moves: LinkedHashSet::new(),
        }
    }

    /// creates a new node for a temp (if we already created a temp for the
    /// temp, returns the node) This function will increase spill cost for
    /// the node by 1 each tiem it is called for the temp
    fn new_node(
        &mut self,
        reg_id: usize,
        ty: NodeType,
        loop_depth: usize,
        context: &LIRFunction,
    ) -> usize {
        let entry = context.get_value(reg_id);

        // if it is the first time, create the node
        if !self.nodes.contains_key(&reg_id) {
            let node = Node {
                temp: reg_id,
                color: None,
                group: RegGroup::from_node(entry).unwrap(),
                spill_cost: 0.0f32,
                freeze_cost: 0f32,
            };

            self.nodes.insert(reg_id, node);
            self.adj_list.insert(reg_id, LinkedHashSet::new());
            self.degree.insert(reg_id, 0);
        }

        // get node
        let node_mut = self.nodes.get_mut(&reg_id).unwrap();
        // increase node spill cost
        node_mut.spill_cost += InterferenceGraph::spillcost_heuristic(ty, loop_depth);

        reg_id
    }

    fn spillcost_heuristic(ty: NodeType, loop_depth: usize) -> f32 {
        const DEF_WEIGHT: f32 = 1f32;
        const USE_WEIGHT: f32 = 1f32;
        const COPY_WEIGHT: f32 = 2f32;

        let loop_depth = loop_depth as i32;

        match ty {
            NodeType::Machine => 0f32,
            NodeType::Def => DEF_WEIGHT * (10f32.powi(loop_depth)),
            NodeType::Use => USE_WEIGHT * (10f32.powi(loop_depth)),
            NodeType::Copy => COPY_WEIGHT * (10f32.powi(loop_depth)),
        }
    }

    /// returns all the nodes in the graph
    pub fn nodes(&self) -> Vec<usize> {
        self.nodes.keys().map(|x| *x).collect()
    }

    /// returns all the moves in the graph
    pub fn moves(&self) -> &LinkedHashSet<Move> {
        &self.moves
    }

    /// adds a move edge between two nodes
    fn add_move(&mut self, src: usize, dst: usize) {
        let src = {
            if is_precolored(src) {
                // get the color for the machine register, e.g. rax for
                // eax/ax/al/ah
                get_color_for_precolored(src)
            } else {
                src
            }
        };

        let dst = {
            if is_precolored(dst) {
                get_color_for_precolored(dst)
            } else {
                dst
            }
        };

        self.moves.insert(Move { from: src, to: dst });
    }

    /// adds an interference edge between two nodes
    pub fn add_edge(&mut self, u: usize, v: usize) {
        // if one of the node is machine register, we add
        // interference edge to its alias
        // e.g. if we have %a - %edi interfered,
        // we will add %a - %rdi interference

        let u = if is_precolored(u) {
            if is_usable(u) {
                get_color_for_precolored(u)
            } else {
                // if it is not usable, we do not need to add an interference
                // edge
                return;
            }
        } else {
            u
        };
        let v = if is_precolored(v) {
            if is_usable(v) {
                get_color_for_precolored(v)
            } else {
                return;
            }
        } else {
            v
        };

        if !self.adj_set.contains(&(u, v)) && u != v {
            trace!("  add edge ({}, {})", u, v);

            self.adj_set.insert((u, v));
            self.adj_set.insert((v, u));

            if !is_precolored(u) {
                self.adj_list.get_mut(&u).unwrap().insert(v);
                let degree = self.get_degree_of(u);
                self.set_degree_of(u, degree + 1);
                trace!("    increase degree of {} to {}", u, degree + 1);
            }
            if !is_precolored(v) {
                self.adj_list.get_mut(&v).unwrap().insert(u);
                let degree = self.get_degree_of(v);
                self.set_degree_of(v, degree + 1);
                trace!("    increase degree of {} to {}", v, degree + 1);
            }
        }
    }

    /// set color for a node
    pub fn color_node(&mut self, reg: usize, color: usize) {
        self.nodes.get_mut(&reg).unwrap().color = Some(color);
    }

    /// is a node colored yet?
    pub fn is_colored(&self, reg: usize) -> bool {
        self.nodes.get(&reg).unwrap().color.is_some()
    }

    /// gets the color of a node
    pub fn get_color_of(&self, reg: usize) -> Option<usize> {
        self.nodes.get(&reg).unwrap().color
    }

    /// gets the reg group of a node
    pub fn get_group_of(&self, reg: usize) -> RegGroup {
        self.nodes.get(&reg).unwrap().group
    }

    /// gets the temporary of a node
    pub fn get_temp_of(&self, reg: usize) -> usize {
        self.nodes.get(&reg).unwrap().temp
    }

    /// gets the spill cost of a node
    pub fn get_spill_cost(&self, reg: usize) -> f32 {
        self.nodes.get(&reg).unwrap().spill_cost
    }

    /// sets the freeze cost of a node
    pub fn set_freeze_cost(&mut self, reg: usize, cost: f32) {
        self.nodes.get_mut(&reg).unwrap().freeze_cost = cost;
    }

    /// gets the freeze cost of a node
    pub fn get_freeze_cost(&self, reg: usize) -> f32 {
        self.nodes.get(&reg).unwrap().freeze_cost
    }

    /// are two nodes the same node?
    fn is_same_node(&self, reg1: usize, reg2: usize) -> bool {
        reg1 == reg2
    }

    /// are two nodes from the same reg group?
    fn is_same_group(&self, reg1: usize, reg2: usize) -> bool {
        self.get_group_of(reg1) == self.get_group_of(reg2)
    }

    /// gets edges from a node
    pub fn get_adj_list(&self, reg: usize) -> &LinkedHashSet<usize> {
        self.adj_list.get(&reg).unwrap()
    }

    pub fn is_in_adj_set(&self, u: usize, v: usize) -> bool {
        self.adj_set.contains(&(u, v))
    }

    /// gets degree of a node (number of edges from the node)
    pub fn get_degree_of(&self, reg: usize) -> usize {
        let ret = *self.degree.get(&reg).unwrap();
        ret
    }

    pub fn set_degree_of(&mut self, reg: usize, degree: usize) {
        trace!("  (set degree({}) = {})", reg, degree);
        self.degree.insert(reg, degree);
    }

    /// prints current graph for debugging (via trace log)
    #[allow(unused_variables)]
    pub fn print(&self, context: &LIRFunction) {
        trace!("");
        trace!("Interference Graph");

        trace!("nodes: ");
        for node in self.nodes.values() {
            trace!("{:?}", node);
        }

        trace!("edges: ");
        for id in self.nodes.keys() {
            let mut s = String::new();
            s.push_str(&format!(
                "edges for {} ({}): ",
                id,
                self.degree.get(id).unwrap()
            ));
            let mut adj = self.get_adj_list(*id).iter();
            if let Some(first) = adj.next() {
                s.push_str(&format!("{:?}", first));
                while let Some(i) = adj.next() {
                    s.push(' ');
                    s.push_str(&format!("{:?}", i));
                }
            }
            trace!("{}", s);
        }
    }
}

/// prints trace during building liveness for debugging?
const TRACE_LIVENESS: bool = false;

/// builds interference graph based on chaitin briggs algorithms
/// reference: Tailoring Graph-coloring Register Allocation For Runtime
/// Compilation
/// - CGO'06, Figure 4
pub fn build_interference_graph_chaitin_briggs(cf: &mut LIRFunction) -> InterferenceGraph {
    build_global_liveness(cf);

    info!("---start building interference graph---");
    let mut ig = InterferenceGraph::new();

    // precolor machine register nodes
    for reg in all_regs().values() {
        let reg_id = c(reg.any_reg_id());
        let node = ig.new_node(reg_id, Machine, 0, cf);
        let precolor = get_color_for_precolored(reg_id);

        ig.color_node(node, precolor);
    }

    // initialize and creates nodes for all the involved temps/regs
    let mc = &cf.code;
    for (id, block) in mc.iter().enumerate() {
        debug!("build graph node for block {}", id);
        let loop_depth: usize = match cf.loop_analysis.as_ref().unwrap().loop_depth.get(&id) {
            Some(depth) => *depth,
            None => 0,
        };
        debug!("loop depth = {}", loop_depth);
        for i in block.instructions.iter() {
            if let Instruction::Move { .. } = i {
                for reg_id in i.get_defs() {
                    let reg_id = c(reg_id);
                    ig.new_node(reg_id, Copy, loop_depth, cf);
                }
                for reg_id in i.get_uses() {
                    let reg_id = c(reg_id);
                    ig.new_node(reg_id, Copy, loop_depth, cf);
                }
            } else if let Instruction::RawIntBinary { .. } = i {
                for reg_id in i.get_defs() {
                    let reg_id = c(reg_id);
                    ig.new_node(reg_id, Copy, loop_depth, cf);
                }
                for reg_id in i.get_uses() {
                    let reg_id = c(reg_id);
                    ig.new_node(reg_id, Copy, loop_depth, cf);
                }
            } else if let Instruction::RawFloatBinary { .. } = i {
                for reg_id in i.get_defs() {
                    let reg_id = c(reg_id);
                    ig.new_node(reg_id, Copy, loop_depth, cf);
                }
                for reg_id in i.get_uses() {
                    let reg_id = c(reg_id);
                    ig.new_node(reg_id, Copy, loop_depth, cf);
                }
            } else {
                for reg_id in i.get_defs() {
                    let reg_id = c(reg_id);
                    ig.new_node(reg_id, Def, loop_depth, cf);
                }
                for reg_id in i.get_uses() {
                    let reg_id = c(reg_id);
                    ig.new_node(reg_id, Use, loop_depth, cf);
                }
            }
            // we separate the case of move nodes, and normal instruction
            // as they yield different spill cost
            // (we prefer spill a node in move instruction
            // as the move instruction can be eliminated)
            /* if mc.is_move(i) {
                for reg_id in mc.get_inst_reg_defines(i) {
                    let reg_id = c(reg_id);
                    ig.new_node(reg_id, Copy, loop_depth, &func.context);
                }

                for reg_id in mc.get_inst_reg_uses(i) {
                    let reg_id = c(reg_id);
                    ig.new_node(reg_id, Copy, loop_depth, &func.context);
                }
            } else {
                for reg_id in mc.get_inst_reg_defines(i) {
                    let reg_id = c(reg_id);
                    ig.new_node(reg_id, Def, loop_depth, &func.context);
                }

                for reg_id in mc.get_inst_reg_uses(i) {
                    let reg_id = c(reg_id);
                    ig.new_node(reg_id, Use, loop_depth, &func.context);
                }
            }*/
        }
    }

    // for each basic block, insert interference edge while reversely traversing
    // instructions
    for (id, block) in cf.code.iter().enumerate() {
        // Current_Live(B) = LiveOut(B)
        /*let mut current_live =
        LinkedHashSet::from_vec(match cf.mc().get_ir_block_liveout(&block) {
            Some(liveout) => liveout.to_vec(),
            None => panic!("cannot find liveout for block {}", block),
        });*/
        let mut current_live = LinkedHashSet::new();
        for liveout in block.liveout.iter() {
            current_live.insert(*liveout);
        }
        let print_set = |set: &LinkedHashSet<usize>| {
            let mut s = String::new();
            let mut iter = set.iter();
            if let Some(first) = iter.next() {
                s.push_str(&format!("{}", first));
                while let Some(i) = iter.next() {
                    s.push(' ');
                    s.push_str(&format!("{}", i));
                }
            }
            trace!("current live: {}", s);
        };

        // for every inst I in reverse order
        for i in block.instructions.iter() {
            let src: Option<usize> = {
                /*if cf.mc().is_move(i) {
                    let src = cf.mc().get_inst_reg_uses(i);
                    let dst = cf.mc().get_inst_reg_defines(i);

                    // src:  reg/imm/mem
                    // dest: reg/mem
                    // we dont care if src/dest is mem
                    if cf.mc().is_using_mem_op(i) {
                        None
                    } else {
                        if src.len() == 1 {
                            let src = c(src[0]);
                            let dst = c(dst[0]);
                            //trace_if!(TRACE_LIVENESS, "add move {} -> {}", src, dst);
                            ig.add_move(src, dst);

                            Some(src)
                        } else {
                            None
                        }
                    }
                } else {
                    None
                }*/
                if let Instruction::Move(dst, src) = i {
                    if let IRNode::Operand(Operand::Memory(_)) = &**dst {
                        None
                    } else if let IRNode::Operand(Operand::Memory(_)) = &**src {
                        None
                    } else if let IRNode::Operand(Operand::Register(r, _)) = &**src {
                        let src = c(*r);
                        let dst = c(dst.any_reg_id());
                        ig.add_move(src, dst);
                        Some(src)
                    } else {
                        None
                    }
                } else {
                    None
                }
            };

            let defines = i.get_defs();
            for d in defines.iter() {
                let d = c(*d);
                current_live.insert(d);
            }
            if TRACE_LIVENESS {
                trace!("after adding defines:");
                print_set(&current_live);
            }

            // for every definition D in I

            for d in defines {
                let d = c(d);
                // add an interference from D to every element E in Current_Live
                // - {D} creating nodes if necessary
                for e in current_live.iter() {
                    if src.is_none() || (src.is_some() && *e != src.unwrap()) {
                        let from = d;
                        let to = *e;

                        if !ig.is_same_node(from, to) && ig.is_same_group(from, to) {
                            if !ig.is_colored(from) {
                                //trace_if!(TRACE_LIVENESS, "add edge between {} and {}", d, *e);
                                ig.add_edge(from, to);
                            }
                            if !ig.is_colored(to) {
                                //trace_if!(TRACE_LIVENESS, "add edge between {} and {}", *e, d);
                                ig.add_edge(to, from);
                            }
                        }
                    }
                }
            }

            // for every definition D in I
            for d in i.get_defs() {
                let d = c(d);
                // remove D from Current_Live
                current_live.remove(&d);
            }
            if TRACE_LIVENESS {
                trace!("removing defines from current live...");
                print_set(&current_live);
            }

            // for every use U in I
            for u in i.get_uses() {
                let u = c(u);
                // add U to Current_live
                current_live.insert(u);
            }
            if TRACE_LIVENESS {
                trace!("adding uses to current live...")
            }
        }
    }

    info!("---finish building interference graph---");
    ig
}

/// builds global liveness for a compiled function
fn build_global_liveness(cf: &mut LIRFunction) {
    info!("---start building live set---");

    // build control flow graphs, treat a whole block as one node in the graph
    let cfg = build_cfg_nodes(cf);
    // do liveness analysis
    global_liveness_analysis(cfg, cf);

    info!("---finish building live set---");
}

/// CFGBlockNode represents a basic block as a whole for global liveness
/// analysis
#[derive(Clone, Debug)]
struct CFGBlockNode {
    block: usize,
    pred: Vec<usize>,
    succ: Vec<usize>,
    uses: Vec<usize>,
    defs: Vec<usize>,
}

/// builds a LinkedHashMap from basic block names to CFGBlockNode
/// We need to collect for each basic block:
/// * predecessors
/// * successors
/// * uses
/// * defs
fn build_cfg_nodes(cf: &mut LIRFunction) -> LinkedHashMap<usize, CFGBlockNode> {
    info!("---local liveness analysis---");
    let mut ret = LinkedHashMap::new();

    // create maps (start_inst -> name) and (end_inst -> name)
    // we will use it to find basic blocks when given a inst index
    let (start_inst_map, end_inst_map) = {
        let mut start_inst_map: LinkedHashMap<usize, usize> = LinkedHashMap::new();
        let mut end_inst_map: LinkedHashMap<usize, usize> = LinkedHashMap::new();
        for block in cf.code.iter() {
            start_inst_map.insert(block.id, block.id);
            end_inst_map.insert(block.id, block.id);
        }

        (start_inst_map, end_inst_map)
    };
    let mut predecessors_: LinkedHashMap<usize, LinkedHashSet<usize>> = LinkedHashMap::new();
    for (id, block) in cf.code.iter().enumerate() {
        if block.instructions.is_empty() {
            continue;
        }
        for target in block.branch_targets().iter().flatten() {
            match predecessors_.get_mut(&target.any_reg_id()) {
                Some(set) => {
                    set.insert(id);
                }
                None => {
                    let mut set = LinkedHashSet::new();
                    set.insert(id);
                    predecessors_.insert(target.any_reg_id(), set);
                }
            }
        }
    }

    let mut successors_: LinkedHashMap<usize, LinkedHashSet<usize>> = LinkedHashMap::new();
    for (id, block) in cf.code.iter().enumerate() {
        if block.instructions.is_empty() {
            continue;
        }

        for target in block.branch_targets().iter().flatten() {
            match successors_.get_mut(&id) {
                Some(set) => {
                    set.insert(target.any_reg_id());
                }
                None => {
                    let mut set = LinkedHashSet::new();
                    set.insert(target.any_reg_id());
                    successors_.insert(id, set);
                }
            }
        }
    }
    // collect info for each basic block
    for (id, block) in cf.code.iter().enumerate() {
        // livein set of this block is what temps this block uses from other blocks
        // defs is what temps this block defines in the block
        let (livein, defs) = {
            // we gradually build livein
            let mut livein = vec![];
            // we need to know all temporaries defined in the block
            // if a temporary is not defined in this block, it is a livein for this block
            let mut all_defined: LinkedHashSet<usize> = LinkedHashSet::new();

            for i in block.instructions.iter() {
                let reg_uses = i.get_uses();

                // if a reg is used but not defined before, it is a live-in
                for reg in reg_uses {
                    let reg = c(reg);
                    if !all_defined.contains(&reg) {
                        livein.push(reg);
                    }
                }

                let reg_defs = i.get_defs();
                for reg in reg_defs {
                    let reg = c(reg);
                    all_defined.insert(reg);
                }
            }

            let defs: Vec<usize> = all_defined.iter().map(|x| *x).collect();

            (livein, defs)
        };

        let preds: Vec<usize> = {
            let mut ret = vec![];

            // predecessors of the first instruction is the predecessors of this block
            if predecessors_.contains_key(&id) {
                for pred in predecessors_.get(&id).unwrap().iter() {
                    match end_inst_map.get(pred) {
                        Some(str) => ret.push(*str),
                        None => {}
                    }
                }
            }

            ret
        };

        let succs: Vec<usize> = {
            let mut ret = vec![];
            if successors_.contains_key(&id) {
                // successors of the last instruction is the successors of this block
                for succ in successors_.get(&id).unwrap().iter() {
                    match start_inst_map.get(succ) {
                        Some(str) => ret.push(*str),
                        None => {}
                    }
                }
            }

            ret
        };

        let node = CFGBlockNode {
            block: block.id,
            pred: preds,
            succ: succs,
            uses: livein,
            defs: defs,
        };

        ret.insert(block.id, node);
    }

    ret
}

/// global analysis, the iterative algorithm to compute livenss until livein/out
/// reaches a fix point
fn global_liveness_analysis(blocks: LinkedHashMap<usize, CFGBlockNode>, cf: &mut LIRFunction) {
    info!("---global liveness analysis---");
    info!("{} blocks", blocks.len());

    // init live in and live out
    let mut livein: LinkedHashMap<usize, LinkedHashSet<usize>> = {
        let mut ret = LinkedHashMap::new();
        for name in blocks.keys() {
            ret.insert(name.clone(), LinkedHashSet::new());
        }
        ret
    };
    let mut liveout: LinkedHashMap<usize, LinkedHashSet<usize>> = {
        let mut ret = LinkedHashMap::new();
        for name in blocks.keys() {
            ret.insert(name.clone(), LinkedHashSet::new());
        }
        ret
    };

    // is the result changed in this iteration?
    let mut is_changed = true;
    // record iteration count
    let mut i = 0;

    while is_changed {
        i += 1;

        // reset
        is_changed = false;

        for node in blocks.keys() {
            let cfg_node = blocks.get(node).unwrap();

            // old livein/out
            let in_set_old = livein.get(node).unwrap().clone();
            let out_set_old = liveout.get(node).unwrap().clone();

            // in <- use + (out - def)
            {
                let mut inset = livein.get_mut(node).unwrap();

                inset.clear();

                // (1) out - def
                add_all(&mut inset, liveout.get(node).unwrap().clone());
                for def in cfg_node.defs.iter() {
                    inset.remove(def);
                }

                // (2) in + (out - def)
                for in_reg in cfg_node.uses.iter() {
                    inset.insert(*in_reg);
                }
            }

            // out[n] <- union(in[s] for every successor s of n)
            {
                let mut outset = liveout.get_mut(node).unwrap();
                outset.clear();

                for s in cfg_node.succ.iter() {
                    add_all(&mut outset, livein.get(s).unwrap().clone());
                }
            }

            // is in/out changed in this iteration?
            let n_changed = !in_set_old.eq(livein.get(node).unwrap())
                || !out_set_old.eq(liveout.get(node).unwrap());

            if true {
                trace!("block {}", node);
                trace!("in(old)  = {:?}", in_set_old);
                trace!("in(new)  = {:?}", livein.get(node).unwrap());
                trace!("out(old) = {:?}", out_set_old);
                trace!("out(new) = {:?}", liveout.get(node).unwrap());
            }

            is_changed = is_changed || n_changed;
        }
    }

    info!("finished in {} iterations", i);

    // set live in and live out
    for block in blocks.keys() {
        let livein: Vec<usize> = livein
            .get(block)
            .unwrap()
            .clone()
            .iter()
            .map(|x| *x)
            .collect();

        cf.code[*block].livein = livein;

        let liveout: Vec<usize> = liveout
            .get(block)
            .unwrap()
            .clone()
            .iter()
            .map(|x| *x)
            .collect();

        cf.code[*block].liveout = liveout;
        //cf.mc_mut().set_ir_block_liveout(block, liveout);
    }
}

fn add_all<V: Eq + std::hash::Hash>(x: &mut LinkedHashSet<V>, mut y: LinkedHashSet<V>) {
    while !y.is_empty() {
        let entry = y.pop_front().unwrap();
        x.insert(entry);
    }
}
