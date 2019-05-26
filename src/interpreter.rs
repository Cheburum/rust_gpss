use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::fmt;

// Инструкции, помеченные (*) содержат в себе указатель, на инструкцию, начиная с которой
// надо начать выполнять, чтобы в стэке оказались нужные аргументы

enum Instructions {
    Generate(usize),  // (*)
    Advance(usize),   // (*)
    Terminate(usize), // (*)
    Print(usize),     // Печатает переменную по адресу usize
    PrintClock,       // просто печатает время
    Transfer(usize),  // операндом является номер инструкции для перехода
    TestVar(usize),   // операндом является номер инструкции для перехода
    SaveValue(usize), // операндом является номер адрес для записи
    Push(usize),      // операндом является адрес, откуда брать элемент для вставки в стэк
}

enum EventType {
    Generate,
    Advance,
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

macro_rules! gpss_type_impl {
    ($($name:ident($type_of:ty)),+) => {

        #[derive(Clone,Copy)]
        enum GpssType {
            $($name($type_of),)+
        }

        impl PartialEq for GpssType {
            fn eq(&self, other: & GpssType) -> bool {
                match self {
                    $(GpssType::$name(self_val) => match other {
                        GpssType::$name(other_val) => self_val == other_val,
                        _ => panic!("Comparing $name and not $name"),
                    },)
                    +
                }
            }
        }

        impl PartialOrd for GpssType {
            fn partial_cmp(&self, other: &GpssType) -> Option<Ordering> {
                match self {
                    $(GpssType::$name(self_val) => match other {
                        GpssType::$name(other_val) => self_val.partial_cmp(other_val),
                        _ => None,
                     },)
                     +
                }
            }
        }

        $(
        impl From<GpssType> for $type_of {
            fn from(item: GpssType) -> Self {
                match &item {
                    GpssType::$name(value) => *value,
                    _ => panic!("Cannot convert type to boolean"),
                }
            }
        }
        )
        +

        impl From<GpssType> for usize {
            fn from(item: GpssType) -> Self {
                match &item {
                    GpssType::UnsignedInteger(value) => *value as usize,
                    _ => panic!("Cannot convert type to boolean"),
                }
            }
        }

        impl fmt::Display for GpssType {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                match self {
                    $(GpssType::$name(val) => write!(f, "{}, {}", stringify!($name), val),)
                    +
                }
            }
        }
        impl GpssType {
            fn empty() -> GpssType {
                GpssType::Boolean(false)
            }
        }
    }
}

gpss_type_impl!(
    Boolean(bool),
    Float(f32),
    Integer(i32),
    Facility(u8),
    UnsignedInteger(u32)
);

#[derive(Clone)]
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

pub struct Interpreter {
    instructions: Vec<Instructions>,
    current_instruction: usize,
    current_transact: Option<Transact>,
    start_entities: u32,
    current_time: u64,
    events: BinaryHeap<Event>,
    memory: Vec<GpssType>,
    stack: Vec<GpssType>,
}

impl Interpreter {
    fn build_interpreter(instructions: Vec<Instructions>, memory: Vec<GpssType>) -> Interpreter {
        Interpreter {
            instructions,
            current_instruction: 0,
            current_transact: None,
            start_entities: 15,
            current_time: 0,
            events: BinaryHeap::new(),
            memory,
            stack: Vec::new(),
        }
    }

    pub fn build_test_interpreter() -> Interpreter {
        use GpssType::*;
        use Instructions::*;
        Self::build_interpreter(
            vec![
                Push(1),      // Какой обьект сохранить (#1)
                SaveValue(0), // Вызов инструкции для сохранения значения
                Push(0),      // Generate возмет время генерации из ячейки #2
                Generate(2),  // Generate в следующий раз вернется на 2-ую иструкцию
                Transfer(5),
                Push(2),
                Advance(5),
                PrintClock,
                Push(3),
                Terminate(8),
                Push(3),
                Terminate(10),
            ],
            vec![
                UnsignedInteger(0),
                Float(0.01),
                Float(0.02),
                UnsignedInteger(1),
            ],
        )
    }

    fn fraction_time_to_int(t: f32) -> u64 {
        (t * 1000.0) as u64
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
                    Instructions::Generate(begin) | Instructions::Advance(begin) => {
                        self.current_instruction = begin;
                        while self.current_instruction < nearest_event.instruction_id {
                            self.process_instruction();
                            self.current_instruction += 1;
                        }
                    }
                    _ => return,
                };

                match self.instructions[nearest_event.instruction_id] {
                    Instructions::Generate(_) => {
                        let time = self.stack_pop_time();
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

    fn stack_pop(&mut self) -> GpssType {
        self.stack.pop().expect("Stack is empty, but trying to pop")
    }

    fn stack_pop_time(&mut self) -> u64 {
        Self::fraction_time_to_int(self.stack_pop().into())
    }

    fn generate(&mut self, time: u64) {
        self.create_event(self.current_instruction, self.current_time + time, None);
        // После того, как создали новое событие
        // ищем и исполняем ближайшее
        self.perform_closest();
    }

    fn advance(&mut self, time: u64) {
        info!("Wake time for ADVANCE {}", self.current_time + time);
        self.create_event(
            self.current_instruction,
            self.current_time + time,
            self.current_transact.clone(),
        );
        self.perform_closest();
    }

    fn terminate(&mut self, count: u32) {
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

    fn print(&mut self, var_id: usize) {
        println!("{}", self.memory[var_id]);
        self.current_instruction += 1;
    }

    fn print_clock(&mut self) {
        println!("Clock {}", self.current_time);
        self.current_instruction += 1;
    }

    fn transfer(&mut self, instruction_id: usize) {
        info!(
            "TRANSFER FROM {} TO {}",
            self.current_instruction, instruction_id
        );
        self.current_instruction = instruction_id;
    }

    fn test_var(&mut self, else_goto: usize, cond_result: bool) {
        if cond_result {
            self.current_instruction += 1;
        } else {
            self.current_instruction = else_goto;
        }
    }

    fn save_value(&mut self, var_id: usize, object: GpssType) {
        if self.memory.len() > var_id {
            self.memory[var_id] = object.clone();
        } else if self.memory.len() == var_id {
            self.memory.push(object.clone());
        } else {
            panic!("Cannot access variable {}", var_id);
        }
        self.current_instruction += 1;
    }

    fn push(&mut self, var_id: usize) {
        info!("Push: {}", self.memory[var_id]);
        self.stack.push(self.memory[var_id]);
        self.current_instruction += 1;
    }

    fn process_instruction(&mut self) {
        match self.instructions[self.current_instruction] {
            //Блоки, требущие подождать. Создаем для них событие в будущем
            Instructions::Generate(_) => self.generate(self.stack_pop_time()),
            Instructions::Advance(_) => self.advance(self.stack_pop_time()),
            //Блоки, не требующие подождать
            Instructions::Terminate(_) => self.terminate(self.stack_pop().into()),
            Instructions::Print(var_id) => self.print(var_id),
            Instructions::PrintClock => self.print_clock(),
            Instructions::Transfer(instruction_id) => self.transfer(instruction_id),
            Instructions::TestVar(else_goto) => self.test_var(else_goto, self.stack_pop().into()),
            Instructions::SaveValue(var_id) => self.save_value(var_id, self.stack_pop()),
            Instructions::Push(var_id) => self.push(var_id),
        };
    }

    pub fn process(&mut self) {
        while self.start_entities > 0 && self.current_instruction < self.instructions.len() {
            self.process_instruction();
        }
    }
}
