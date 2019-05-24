use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::fmt;

enum UnaryOperator {
    IsTrue,
    IsFalse,
    Utilised,
    NotUtilised,
}

enum BinaryOperator {
    Greater,
    Less,
    Equal,
    NotEqual,
}

enum LogicOperator {
    Binary(BinaryOperator, usize, usize),
    Unary(UnaryOperator, usize),
}

enum Instructions<'a> {
    Generate(u64),
    Advance(u64),
    Terminate(u32),
    PrintParam(u8),
    PrintText(&'a str),
    PrintClock,
    Transfer(usize),
    Test(LogicOperator,usize),
}

struct Event {
    instruction_id: usize,
    wake_time: u64,
    transact: Option<Transact>,
}

impl Ord for Event {
    fn cmp(&self, other: &Event) -> Ordering {
        other.wake_time.cmp(&self.wake_time)
    }
}

impl PartialOrd for Event {
    fn partial_cmp(&self, other: &Event) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Event {
    fn eq(&self, other: &Event) -> bool {
        self.wake_time == other.wake_time
    }
}

impl Eq for Event {}

#[derive(Copy, Clone)]
enum GpssType {
    Boolean(bool),
    Float(f32),
    Integer(i32),
    Facility(u8),
}

impl PartialEq for GpssType {
    fn eq(&self, other: &GpssType) -> bool {
        match self {
            GpssType::Boolean(self_val) => match other {
                GpssType::Boolean(other_val) => self_val == other_val,
                _ => panic!("Comparing boolean and not boolean"),
            },
            GpssType::Float(self_val) => match other {
                GpssType::Float(other_val) => self_val == other_val,
                _ => panic!("Comparing float and not float"),
            },
            GpssType::Integer(self_val) => match other {
                GpssType::Integer(other_val) => self_val == other_val,
                _ => panic!("Comparing integer and not integer"),
            },
            _ => panic!("Cannot compare facilities"),
        }
    }
}

impl PartialOrd for GpssType {
    fn partial_cmp(&self, other: &GpssType) -> Option<Ordering> {
        match self {
            GpssType::Boolean(self_val) => match other {
                GpssType::Boolean(other_val) => self_val.partial_cmp(other_val),
                _ => None,
            },
            GpssType::Float(self_val) => match other {
                GpssType::Float(other_val) => self_val.partial_cmp(other_val),
                _ => panic!("Comparing float and not float"),
            },
            GpssType::Integer(self_val) => match other {
                GpssType::Integer(other_val) => self_val.partial_cmp(other_val),
                _ => panic!("Comparing integer and not integer"),
            },
            _ => panic!("Cannot compare facilities"),
        }
    }
}

impl From<GpssType> for bool {
    fn from(item: GpssType) -> Self {
        match item {
            GpssType::Boolean(value) => value,
            _ => panic!("Cannot convert type to boolean"),
        }
    }
}

impl fmt::Display for GpssType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GpssType::Boolean(b) => write!(f, "Boolean, {}", b),
            GpssType::Float(fl) => write!(f, "Float, {}", fl),
            GpssType::Integer(int) => write!(f, "Int, {}", int),
            GpssType::Facility(count) => write!(f, "Facility with: {} items", count),
        }
    }
}

impl GpssType {
    fn empty() -> GpssType {
        GpssType::Boolean(false)
    }
}

#[derive(Copy, Clone)]
struct Transact {
    params: [GpssType; 16],
}

impl Transact {
    fn empty() -> Transact {
        Transact {
            params: array![|_| GpssType::empty();16],
        }
    }
}

pub struct Interpreter<'a> {
    instructions: Vec<Instructions<'a>>,
    current_instruction: usize,
    current_transact: Option<Transact>,
    start_entities: u32,
    current_time: u64,
    events: BinaryHeap<Event>,
    variables: Vec<GpssType>,
}

impl<'a> Interpreter<'a> {
    pub fn build_test_interpreter() -> Interpreter<'a> {
        Interpreter {
            instructions: vec![
                //Instructions::PRINT_CLOCK,
                Instructions::Generate(2),
                //Instructions::PRINT_CLOCK,
                //Instructions::PRINT_PARAM(0),
                Instructions::Transfer(3),
                Instructions::Advance(1),
                //Instructions::PRINT_CLOCK,
                //Instructions::PRINT_TEXT("Object destroyed".into()),
                Instructions::Terminate(1),
            ],
            current_instruction: 0,
            current_transact: None,
            start_entities: 15,
            current_time: 0,
            events: BinaryHeap::new(),
            variables: Vec::new(),
        }
    }

    fn fraction_time_to_int(t: f32) -> u64 {
        (t * 100.0) as u64
    }

    fn perform_closest(&mut self) {
        // В этом match идет исполнение кода для откладываемых событий
        // Исполняем ближайшее событие, если оно есть
        match self.events.pop() {
            Some(nearest_event) => {
                self.current_time = nearest_event.wake_time;
                info!("Woke up at {}", self.current_time);
                self.current_transact = nearest_event.transact;
                match self.instructions[nearest_event.instruction_id] {
                    Instructions::Generate(time) => {
                        info!("DOING GENERATE");
                        let mut new_transact = Transact::empty();
                        new_transact.params[0] = GpssType::Integer(rand::random::<i32>());
                        self.current_transact = Some(new_transact);
                        // после генерации текущего транзакта, надо запланировать генерацию следующего
                        self.create_event(
                            nearest_event.instruction_id,
                            self.current_time + time,
                            None,
                        );
                        self.current_instruction = nearest_event.instruction_id + 1;
                    }
                    Instructions::Advance(_) => {
                        info!("DOING ADVANCE");
                        self.current_instruction = nearest_event.instruction_id + 1;
                    }
                    _ => {
                        self.current_instruction = nearest_event.instruction_id + 1;
                    }
                }
            }
            None => {}
        }
    }

    fn create_event(&mut self, instruction_id: usize, wake_time: u64, transact: Option<Transact>) {
        self.events.push(Event {
            instruction_id,
            wake_time,
            transact,
        });
    }

    fn is_facility_utilised(fac: GpssType) -> Option<bool> {
        match fac {
            GpssType::Facility(count) => Some(count != 0),
            _ => None,
        }
    }

    pub fn process(&mut self) {
        while self.start_entities > 0 && self.current_instruction < self.instructions.len() {
            match &self.instructions[self.current_instruction] {
                //Блоки, требущие подождать. Создаем для них событие в будущем
                Instructions::Generate(time) => {
                    self.create_event(self.current_instruction, self.current_time + time, None);
                    // После того, как создали новое событие
                    // ищем и исполняем ближайшее
                    self.perform_closest();
                }
                Instructions::Advance(time) => {
                    info!("Wake time for ADVANCE {}", self.current_time + time);
                    self.create_event(
                        self.current_instruction,
                        self.current_time + time,
                        self.current_transact.clone(),
                    );
                    self.perform_closest();
                }
                //Блоки, не требующие подождать
                Instructions::Terminate(count) => {
                    info!("TERMINATE {}", count);
                    self.start_entities -= count;
                    self.current_transact = None;
                    if self.events.len() > 0 {
                        self.perform_closest();
                    } else {
                        info!("STOP");
                        self.current_instruction += 1;
                    }
                }
                Instructions::PrintParam(param_id) => {
                    println!(
                        "{}",
                        self.current_transact.unwrap().params[*param_id as usize]
                    );
                    self.current_instruction += 1;
                }
                Instructions::PrintText(param) => {
                    println!("{}", param);
                    self.current_instruction += 1;
                }
                Instructions::PrintClock => {
                    println!("Clock {}", self.current_time);
                    self.current_instruction += 1;
                }
                Instructions::Transfer(instruction_id) => {
                    info!(
                        "TRANSFER FROM {} TO {}",
                        self.current_instruction, instruction_id
                    );
                    self.current_instruction = *instruction_id;
                }
                Instructions::Test(operator, else_goto) => {
                    let cond_result: bool = match operator {
                        LogicOperator::Binary(bin_operator, a, b) => match bin_operator {
                            BinaryOperator::Equal => self.variables[*a] == self.variables[*b],
                            BinaryOperator::Greater => self.variables[*a] > self.variables[*b],
                            BinaryOperator::Less => self.variables[*a] < self.variables[*b],
                            BinaryOperator::NotEqual => self.variables[*a] != self.variables[*b],
                        },
                        LogicOperator::Unary(un_operator, a) => match un_operator {
                            UnaryOperator::IsFalse => {
                                let res: bool = self.variables[*a].into();
                                !res
                            }
                            UnaryOperator::IsTrue => self.variables[*a].into(),
                            UnaryOperator::NotUtilised => {
                                !Self::is_facility_utilised(self.variables[*a])
                                    .expect("Not a facility")
                            }
                            UnaryOperator::Utilised => {
                                Self::is_facility_utilised(self.variables[*a])
                                    .expect("Not a facility")
                            }
                        },
                    };
                    if cond_result{
                        self.current_instruction += 1;
                    }else{
                        self.current_instruction = *else_goto;
                    }
                }
            };
        }
    }
}