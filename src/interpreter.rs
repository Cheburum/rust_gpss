use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::fmt;

/// Instructions, marked with (*) contain pointer(usize) to instruction
/// from what it will be executed, to have proper arguments in stack

enum Instructions {
    /// (*) pops time interval to generate from stack
    Generate(usize),
    /// (*) pops time interval to wait from stack
    Advance(usize),
    /// (*) pops terminate count from stack
    Terminate(usize),
    /// Prints object by its address
    Print(usize),
    /// Prints clock
    PrintClock,
    /// Operand is pointer to instruction
    Transfer(usize),
    /// Operand is pointer to instruction for false branch. Pops condition(GppsType::Boolean) from stack.
    TestVar(usize),
    /// Operand is a pointer to memory. Takes object from stack and writes it to memory.
    SaveValue(usize),
    /// Operand is a pointer to memory. Pushes object from memory to stack.
    Push(usize),
}

/// Event info, which must be handled to execute it lates
struct Event {
    /// pointer to instruction
    instruction_id: usize,
    /// when will event be executed
    wake_time: u64,
    /// transact related to event
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
        /// Types you can use as properties of transacts
        /// or as variables
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

/// Transact. Has 16 properties.
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

/// State of interpreter
pub struct Interpreter {
    /// Instructions to execute(program)
    instructions: Vec<Instructions>,
    /// Pointer to current instruction
    current_instruction: usize,
    /// Current transact. For current context.
    current_transact: Option<Transact>,
    /// Number START. Program stops when START becomes zero.
    start_entities: u32,
    /// Clock of simulation
    current_time: u64,
    /// Events, sorted by time of awake
    events: BinaryHeap<Event>,
    /// Global memory
    memory: Vec<GpssType>,
    /// Stack
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

    /// Program example
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
                Push(4),
                TestVar(10),
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
                Boolean(false),
            ],
        )
    }

    fn fraction_time_to_int(t: f32) -> u64 {
        (t * 1000.0) as u64
    }

    fn int_time_to_fraction(t: u64) -> f32 {
        t as f32 / 1000.0
    }

    fn is_facility_utilised(fac: GpssType) -> Option<bool> {
        match fac {
            GpssType::Facility(count) => Some(count != 0),
            _ => None,
        }
    }

    /// Pops object from stack. Panics if stack is empty.
    fn stack_pop(&mut self) -> GpssType {
        self.stack.pop().expect("Stack is empty, but trying to pop")
    }

    fn stack_pop_time(&mut self) -> u64 {
        Self::fraction_time_to_int(self.stack_pop().into())
    }

    fn generate(&mut self, time: u64) {
        info!("Wake time for GENERATE {}", self.current_time + time);
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
        if self.events.len() > 0 && self.start_entities > 0 {
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
        info!("Condition is {}", cond_result);
        if cond_result {
            self.current_instruction += 1;
        } else {
            self.current_instruction = else_goto;
        }
    }

    fn save_value(&mut self, var_id: usize, object: GpssType) {
        info!("Saving value {} to {}", object, var_id);
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

    /// Executes commands from start to end. Excluding end.
    fn process_from_to(&mut self, start: usize, end: usize) {
        self.current_instruction = start;
        while self.current_instruction < end {
            self.process_instruction();
            self.current_instruction += 1;
        }
    }

    /// Executes closest event
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
                        self.process_from_to(begin, nearest_event.instruction_id);
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

    /// Schedules event in future
    fn create_event(&mut self, instruction_id: usize, wake_time: u64, transact: Option<Transact>) {
        self.events.push(Event {
            instruction_id,
            wake_time,
            transact,
        });
    }

    /// Executes current instruction
    fn process_instruction(&mut self) {
        match self.instructions[self.current_instruction] {
            //Блоки, требущие подождать. Создаем для них событие в будущем
            Instructions::Generate(_) => {
                let time = self.stack_pop_time();
                self.generate(time);
            }
            Instructions::Advance(_) => {
                let time = self.stack_pop_time();
                self.advance(time);
            }
            //Блоки, не требующие подождать
            Instructions::Terminate(_) => {
                let count = self.stack_pop().into();
                self.terminate(count);
            }
            Instructions::Print(var_id) => self.print(var_id),
            Instructions::PrintClock => self.print_clock(),
            Instructions::Transfer(instruction_id) => self.transfer(instruction_id),
            Instructions::TestVar(else_goto) => {
                let cond_result = self.stack_pop().into();
                self.test_var(else_goto, cond_result)
            }
            Instructions::SaveValue(var_id) => {
                let object = self.stack_pop();
                self.save_value(var_id, object);
            }
            Instructions::Push(var_id) => self.push(var_id),
        };
    }

    /// Interpretation
    pub fn process(&mut self) {
        while self.start_entities > 0 && self.current_instruction < self.instructions.len() {
            self.process_instruction();
        }
    }
}
