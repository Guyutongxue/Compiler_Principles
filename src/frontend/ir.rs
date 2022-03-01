use koopa::ir::builder::{BasicBlockBuilder, GlobalInstBuilder, LocalInstBuilder, ValueBuilder};
use koopa::ir::dfg::DataFlowGraph;
use koopa::ir::layout::{InstList, Layout};
use koopa::ir::{BasicBlock, Function, FunctionData, Program, Type, TypeKind, Value, ValueKind};
use std::borrow::BorrowMut;

use super::ast::{CompUnit, Decl, Declarator, FuncDecl, Initializer, InitializerLike, TypeSpec};
use super::consteval::{Eval, ConstValue};
use super::error::CompileError;
#[allow(unused_imports)]
use super::error::{PushKeyError, UnimplementedError};
use super::stmt::{self, get_layout};
use super::symbol::{Symbol, SymbolTable};
use super::ty::{self, TyUtils};
use crate::frontend::consteval::EvalError;
use crate::Result;

pub struct GenerateContext<'a> {
  pub program: &'a mut Program,
  pub func: Function,
  pub bb: Option<BasicBlock>,
  pub symbol: SymbolTable,

  next_bb_no: Box<dyn Iterator<Item = i32>>,

  /// 循环中 break/continue 跳转位置
  pub loop_jump_pt: Vec<(BasicBlock, BasicBlock)>,
}

fn generate_param_list(params: &Vec<Box<Declarator>>) -> Result<Vec<(Option<String>, Type)>> {
  let mut ir = vec![];
  for param in params {
    let (tys, name) = ty::parse(param.as_ref())?;
    ir.push((Some(format!("@{}", name)), tys.to_ir()));
  }
  Ok(ir)
}

impl<'a> GenerateContext<'a> {
  pub fn new(program: &'a mut Program, func_ast: &FuncDecl) -> Result<Self> {
    let func_ir_name = format!("@{}", func_ast.ident);
    let func_ir_param = generate_param_list(&func_ast.params)?;
    let func_ir_type = match func_ast.func_type {
      TypeSpec::Int => Type::get_i32(),
      TypeSpec::Void => Type::get_unit(),
    };

    let func = program.new_func(FunctionData::with_param_names(
      func_ir_name,
      func_ir_param,
      func_ir_type,
    ));

    let mut this = Self {
      program: program,
      func,
      bb: None,
      symbol: SymbolTable::new(),
      next_bb_no: Box::new(0..),
      loop_jump_pt: vec![],
    };

    if func_ast.body.is_some() {
      // %entry basic block
      let entry = this.add_bb()?;
      this.bb = Some(entry);

      // Store parameters to local variable
      for (i, param) in func_ast.params.iter().enumerate() {
        let (_, name) = ty::parse(param.as_ref())?;
        let param = this.program.func(this.func).params()[i];
        let param_type = this.dfg().value(param).ty().clone();

        let alloc = this.dfg().new_value().alloc(param_type);
        let store = this.dfg().new_value().store(param, alloc);

        this.add_inst(alloc)?;
        this.add_inst(store)?;

        if !this.symbol.insert(&name, Symbol::Var(alloc)) {
          Err(CompileError::Redefinition(name.into()))?;
        }
      }
    }
    Ok(this)
  }

  pub fn dfg(&mut self) -> &mut DataFlowGraph {
    self.program.func_mut(self.func).dfg_mut()
  }
  fn layout(&mut self) -> &mut Layout {
    self.program.func_mut(self.func).layout_mut()
  }

  pub fn add_bb(&mut self) -> Result<BasicBlock> {
    let name = format!("%bb{}", self.next_bb_no.next().unwrap());
    let bb = self.dfg().new_bb().basic_block(Some(name));
    self
      .layout()
      .bbs_mut()
      .push_key_back(bb)
      .map_err(|k| PushKeyError(Box::new(k)))?;
    Ok(bb)
  }

  pub fn insts(&mut self, bb: BasicBlock) -> &mut InstList {
    self.layout().bb_mut(bb).insts_mut()
  }

  pub fn add_inst(&mut self, value: Value) -> Result<()> {
    if let Some(bb) = self.bb {
      self.insts(bb).push_key_back(value).map_err(|k| {
        let vd = self.dfg().value(k).clone();
        PushKeyError(Box::new(vd))
      })?;
    }
    Ok(())
  }

  pub fn switch_bb(&mut self, final_inst: Value, new_bb: Option<BasicBlock>) -> Result<()> {
    self.add_inst(final_inst)?;
    self.bb = new_bb;
    Ok(())
  }
}

pub fn generate_program(ast: CompUnit) -> Result<Program> {
  // 参考 https://github.com/pku-minic/sysy-runtime-lib/blob/master/src/sysy.h
  let prelude = r#"
decl @getint(): i32
decl @getch(): i32
decl @getarray(*i32): i32
decl @putint(i32): i32
decl @putch(i32): i32
decl @putarray(i32, *i32): i32
decl @starttime(): i32
decl @stoptime(): i32
"#;
  let driver = koopa::front::Driver::from(prelude);
  let mut program = driver.generate_program().unwrap();

  for decl in &ast {
    match decl {
      Decl::Func(decl) => {
        let name = &decl.ident;
        let mut context = GenerateContext::new(&mut program, &decl)?;

        if let Some(block) = &decl.body {
          // Function definition
          if !SymbolTable::insert_global_def(name, Symbol::Func(context.func)) {
            Err(CompileError::Redefinition(decl.ident.clone()))?;
          }
          for i in block.iter() {
            stmt::generate(i, &mut context)?;
          }
        } else {
          // Function declaration
          SymbolTable::insert_global_decl(name, Symbol::Func(context.func));
        }
      }
      Decl::Var(declaration) => {
        if declaration.ty == TypeSpec::Void {
          Err(CompileError::IllegalVoid)?;
        }
        for (decl, init) in &declaration.list {
          let (tys, name) = ty::parse(decl.as_ref())?;
          println!("{:#?}", tys);
          if declaration.is_const {
            let init = init
              .as_ref()
              .ok_or(CompileError::InitializerRequired(name.into()))?;
            let value = match init.eval(None) {
              Err(e) => Err({
                match e {
                  EvalError::NotConstexpr => CompileError::ConstexprRequired("全局常量初始化器"),
                  EvalError::CompileError(e) => e,
                }
              })?,
              Ok(exp) => match &exp {
                InitializerLike::Simple(exp) => ConstValue::int(*exp),
                InitializerLike::Aggregate(_) => {
                  let size = tys.get_array_size();
                  let layout = get_layout(&size, &exp, &(|| 0))?;
                  ConstValue::from(size, layout)
                }
              },
            };
            if !SymbolTable::insert_global_def(&name, Symbol::Const(value)) {
              Err(CompileError::Redefinition(name.into()))?;
            }
          } else {
            let value = match init {
              Some(init) => match init.eval(None) {
                Err(e) => Err({
                  match e {
                    EvalError::NotConstexpr => CompileError::ConstexprRequired("全局变量初始化器"),
                    EvalError::CompileError(e) => e,
                  }
                })?,
                Ok(exp) => match &exp {
                  InitializerLike::Simple(int) => program.new_value().integer(*int),
                  InitializerLike::Aggregate(_) => {
                    let size = tys.get_array_size();
                    let layout = get_layout(&size, &exp, &(|| 0))?;
                    println!("{:#?}", &layout);
                    let const_value = ConstValue::from(size, layout);
                    const_value.to_ir(&mut program)
                  }
                }
              },
              None => program.new_value().zero_init(tys.to_ir()),
            };
            let alloc = program.new_value().global_alloc(value);
            program
              .borrow_mut()
              .set_value_name(alloc, Some(format!("%{}", name)));
            if !SymbolTable::insert_global_def(&name, Symbol::Var(alloc)) {
              Err(CompileError::Redefinition(name.into()))?;
            }
          }
        }
      }
    }
  }

  for (_, fd) in program.funcs_mut().iter_mut() {
    add_extra_ret(fd);
  }

  Ok(program)
}

/// Add `ret` value for bbs not ends with `ret`
fn add_extra_ret(fd: &mut FunctionData) {
  let mut need_ret_bbs = vec![];
  for (bb, bbn) in fd.layout().bbs() {
    if let Some(inst) = bbn.insts().back_key() {
      let kind = fd.dfg().value(*inst).kind();
      if matches!(kind, ValueKind::Return(_))
        || matches!(kind, ValueKind::Jump(_))
        || matches!(kind, ValueKind::Branch(_))
      {
        continue;
      }
      need_ret_bbs.push(*bb);
    } else {
      need_ret_bbs.push(*bb);
    }
  }
  for bb in need_ret_bbs {
    if let TypeKind::Function(_, ret_type) = fd.ty().kind() {
      let ret = if Type::is_i32(ret_type) {
        let retval = fd.dfg_mut().new_value().integer(0);
        fd.dfg_mut().new_value().ret(Some(retval))
      } else {
        fd.dfg_mut().new_value().ret(None)
      };
      fd.layout_mut()
        .bb_mut(bb)
        .insts_mut()
        .push_key_back(ret)
        .unwrap();
    }
  }
}

trait ToIr {
  fn to_ir(&self, program: &mut Program) -> Value;
}

impl ToIr for ConstValue {
  fn to_ir(&self, program: &mut Program) -> Value {
    if self.size.len() == 0 {
      return program.new_value().integer(self.data[0]);
    } else {
      let mut values = vec![];
      for i in 0..self.size[0] {
        values.push(self.item(i).unwrap().to_ir(program));
      }
      program.new_value().aggregate(values)
    }
  }
}
