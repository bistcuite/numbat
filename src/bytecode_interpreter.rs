use crate::ast::{BinaryOperator, Command, Expression, Statement};
use crate::dimension::DimensionRegistry;
use crate::interpreter::{Interpreter, InterpreterError, InterpreterResult, Result};
use crate::vm::{Op, Vm};

pub struct BytecodeInterpreter {
    vm: Vm,
    registry: DimensionRegistry,
}

impl BytecodeInterpreter {
    fn compile_expression(&mut self, expr: &Expression) -> Result<()> {
        match expr {
            Expression::Scalar(n) => {
                let index = self.vm.add_constant(n.to_f64());
                self.vm.add_op1(Op::Constant, index);
            }
            Expression::Identifier(identifier) => {
                let identifier_idx = self.vm.add_identifier(identifier);
                self.vm.add_op1(Op::GetVariable, identifier_idx);
            }
            Expression::Negate(rhs) => {
                self.compile_expression(rhs)?;
                self.vm.add_op(Op::Negate);
            }
            Expression::BinaryOperator(operator, lhs, rhs) => {
                self.compile_expression(lhs)?;
                self.compile_expression(rhs)?;

                let op = match operator {
                    BinaryOperator::Add => Op::Add,
                    BinaryOperator::Sub => Op::Subtract,
                    BinaryOperator::Mul => Op::Multiply,
                    BinaryOperator::Div => Op::Divide,
                    BinaryOperator::ConvertTo => todo!(),
                };
                self.vm.add_op(op);
            }
        };

        Ok(())
    }

    fn compile_statement(&mut self, stmt: &Statement) -> Result<()> {
        match stmt {
            Statement::Expression(expr) => {
                self.compile_expression(&expr)?;
                self.vm.add_op(Op::Return);
            }
            Statement::Command(Command::List) => {
                self.vm.add_op(Op::List);
            }
            Statement::Command(Command::Exit) => {
                self.vm.add_op(Op::Exit);
            }
            Statement::DeclareVariable(identifier, expr) => {
                self.compile_expression(&expr)?;
                let identifier_idx = self.vm.add_identifier(identifier);
                self.vm.add_op1(Op::SetVariable, identifier_idx);
            }
            Statement::DeclareDimension(name, exprs) => {
                if exprs.is_empty() {
                    self.registry
                        .add_base_dimension(name)
                        .map_err(InterpreterError::DimensionRegistryError)?;
                } else {
                    let base_representation = self
                        .registry
                        .add_derived_dimension(name, &exprs[0])
                        .map_err(InterpreterError::DimensionRegistryError)?
                        .clone();

                    for alternative_expr in &exprs[1..] {
                        let alternative_base_representation = self
                            .registry
                            .get_base_representation(alternative_expr)
                            .map_err(InterpreterError::DimensionRegistryError)?;
                        if alternative_base_representation != *base_representation {
                            return Err(
                                InterpreterError::IncompatibleAlternativeDimensionExpression(
                                    name.clone(),
                                ),
                            );
                        }
                    }
                }

                /*println!(
                    "{:?}",
                    self.registry.get_base_representation(
                        &crate::ast::DimensionExpression::Dimension(name.clone())
                    )
                );*/

                self.vm.add_op(Op::List); // TODO
            }
            Statement::DeclareUnit(name, expr) => {
                let base_rep = self
                    .registry
                    .get_base_representation(expr)
                    .map_err(InterpreterError::DimensionRegistryError)?;

                // TODO

                self.vm.add_op(Op::List); // TODO
            }
        }

        Ok(())
    }

    fn run(&mut self) -> Result<InterpreterResult> {
        self.vm.disassemble();

        let result = self.vm.run();

        self.vm.debug();

        result
    }
}

impl Interpreter for BytecodeInterpreter {
    fn new() -> Self {
        Self {
            vm: Vm::new(),
            registry: DimensionRegistry::default(),
        }
    }

    fn interpret_statement(&mut self, statement: &Statement) -> Result<InterpreterResult> {
        self.compile_statement(statement)?;
        self.run()
    }
}