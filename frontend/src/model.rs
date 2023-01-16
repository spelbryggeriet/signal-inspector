use std::{
    iter,
    ops::{Index, IndexMut, Not},
};

use im::Vector;

#[derive(Clone, PartialEq)]
pub struct CellGrid {
    cells: Vector<Vector<Option<CellShape>>>,
}

impl CellGrid {
    pub fn new(size: usize) -> Self {
        let mut rows = Vector::new();
        rows.extend(
            iter::repeat({
                let mut cells = Vector::new();
                cells.extend(iter::repeat(None).take(size));
                cells
            })
            .take(size),
        );
        Self { cells: rows }
    }

    pub fn size(&self) -> usize {
        self.cells.len()
    }

    fn solved_row(&self) -> Option<usize> {
        self.cells.iter().position(|row| {
            row.get(0)
                .map(|first| first.is_some() && row.iter().all(|elem| elem == first))
                .unwrap_or(false)
        })
    }

    fn solved_col(&self) -> Option<usize> {
        let size = self.size();

        (0..size).position(|col| {
            self.cells
                .get(0)
                .and_then(|row| row.get(col))
                .map(|first| {
                    first.is_some()
                        && self
                            .cells
                            .iter()
                            .all(|row| row.get(col).map(|elem| elem == first).unwrap_or(false))
                })
                .unwrap_or(false)
        })
    }

    fn solved_diag(&self) -> Option<usize> {
        let size = self.size();
        (0..=1).position(|k| {
            self.cells
                .get(0)
                .and_then(|row| row.get(if k == 0 { 0 } else { size - 1 }))
                .map(|first| {
                    first.is_some()
                        && self.cells.iter().enumerate().all(|(i, row)| {
                            row.get(if k == 0 { i } else { size - 1 - i })
                                .map(|elem| elem == first)
                                .unwrap_or(false)
                        })
                })
                .unwrap_or(false)
        })
    }

    pub fn is_full(&self) -> bool {
        self.cells
            .iter()
            .all(|row| row.iter().all(|cell| cell.is_some()))
    }

    pub fn is_solved(&self) -> bool {
        self.solved_row().is_some() || self.solved_col().is_some() || self.solved_diag().is_some()
    }

    pub fn clear_non_solved(&mut self) {
        if let Some(i_solved) = self.solved_row() {
            self.cells
                .iter_mut()
                .enumerate()
                .filter(|(i, _)| *i != i_solved)
                .for_each(|(_, row)| {
                    row.iter_mut().for_each(|cell| {
                        cell.take();
                    })
                });
            return;
        }

        if let Some(j_solved) = self.solved_col() {
            self.cells.iter_mut().for_each(|row| {
                row.iter_mut()
                    .enumerate()
                    .filter(|(j, _)| *j != j_solved)
                    .for_each(|(_, cell)| {
                        cell.take();
                    })
            });
            return;
        }

        if let Some(k_solved) = self.solved_diag() {
            let size = self.size();
            self.cells.iter_mut().enumerate().for_each(|(i, row)| {
                row.iter_mut().enumerate().for_each(|(j, cell)| {
                    if k_solved == 0 && i != j || k_solved == 1 && i + j != size - 1 {
                        cell.take();
                    }
                })
            });
            return;
        }

        self.clear_all();
    }

    pub fn clear_all(&mut self) {
        self.cells.iter_mut().for_each(|row| {
            row.iter_mut().for_each(|cell| {
                cell.take();
            })
        });
    }
}

impl Index<usize> for CellGrid {
    type Output = <Vector<Vector<Option<CellShape>>> as Index<usize>>::Output;

    fn index(&self, index: usize) -> &Self::Output {
        &self.cells[index]
    }
}

impl IndexMut<usize> for CellGrid {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.cells[index]
    }
}

#[derive(PartialEq, Clone, Copy)]
pub enum CellShape {
    Circle,
    Cross,
}

impl Not for CellShape {
    type Output = Self;

    fn not(self) -> Self::Output {
        match self {
            Self::Circle => Self::Cross,
            Self::Cross => Self::Circle,
        }
    }
}
