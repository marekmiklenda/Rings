use std::io::Cursor;

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

use root::{CompileError, ProgramEnvironment, RuntimeError};
use root::RuntimeError::{Halt, StdinReadError};

use crate::rings::root;

pub type IFuncFn = fn(env: &mut ProgramEnvironment, instr: Cursor<&mut [u8]>) -> Result<(), RuntimeError>;
pub type IArgsFn = fn(&mut Vec<u8>, &[&str], &[(String, u16)]) -> Result<(), CompileError>;

pub const I_KEYS: [&str; 16] = ["mkr", "put", "rot", "swp", "inp", "out", "err", "add", "sub", "mul", "div", "jmp", "jeq", "jgt", "jlt", "hlt"];
pub const I_FUNC: [IFuncFn; 16] = [impl_mkr, impl_put, impl_rot, impl_swp, impl_inp, impl_out, impl_err, impl_add, impl_sub, impl_mul, impl_div, impl_jmp, impl_jeq, impl_jgt, impl_jlt, impl_hlt];
pub const I_ARGS: [IArgsFn; 16] = [args_mkr, args_2u8, args_2u8, args_2u8, args_1u8, args_1u8, args_1u8, args_3u8, args_3u8, args_3u8, args_3u8, args_lbl, args_cjm, args_cjm, args_cjm, args_1u8];
pub const I_SIZE: [usize; 16] = [1, 2, 2, 2, 1, 1, 1, 3, 3, 3, 3, 2, 4, 4, 4, 1];

fn read_ring(env: &ProgramEnvironment, args: &mut Cursor<&mut [u8]>) -> Result<u8, RuntimeError> {
    let val = args.read_u8().unwrap();
    if val > env.len() { return Err(RuntimeError::IndexOutOfBounds { got: val as usize, max: (env.len() - 1) as usize }); }

    Ok(val)
}

fn read_rings<'a>(len: u8, env: &'a mut ProgramEnvironment, args: &'a mut Cursor<&mut [u8]>) -> Result<Vec<u8>, RuntimeError> {
    let mut result: Vec<u8> = vec![0; len as usize];

    for i in 0..len as usize {
        match read_ring(env, args) {
            Ok(val) => result[i] = val,
            Err(e) => { return Err(e); }
        }
    }

    Ok(result)
}

fn dyn_args(num: u8, offset: u8, out: &mut Vec<u8>, args: &[&str]) -> Result<(), CompileError> {
    for i in offset as usize..(num + offset) as usize {
        if let Ok(val) = args[i].parse::<u8>() {
            // Not the nicest, but gets rid of the compiler warning and doesn't completely discard the error, if it were to theoretically happen.
            if let Err(e) = out.write_u8(val) { panic!("{:?}", e) };
            continue;
        }

        return Err(CompileError::TypeMismatch { expected: String::from("u8"), got: String::from(args[i]) });
    }

    Ok(())
}

/***********************
 * Args implementation *
 ***********************/
fn args_1u8(out: &mut Vec<u8>, args: &[&str], _: &[(String, u16)]) -> Result<(), CompileError> {
    if args.len() != 1 { return Err(CompileError::SyntaxError); }
    dyn_args(1, 0, out, args)
}

fn args_2u8(out: &mut Vec<u8>, args: &[&str], _: &[(String, u16)]) -> Result<(), CompileError> {
    if args.len() != 2 { return Err(CompileError::SyntaxError); }
    dyn_args(2, 0, out, args)
}

fn args_3u8(out: &mut Vec<u8>, args: &[&str], _: &[(String, u16)]) -> Result<(), CompileError> {
    if args.len() != 3 { return Err(CompileError::SyntaxError); }
    dyn_args(3, 0, out, args)
}

fn args_mkr(out: &mut Vec<u8>, args: &[&str], _: &[(String, u16)]) -> Result<(), CompileError> {
    use CompileError::{SyntaxError, InvalidValue, TypeMismatch};

    if args.len() != 1 { return Err(SyntaxError); }

    if let Ok(len) = args[0].parse::<u8>() {
        if len == 0 { return Err(InvalidValue(len as usize)); }
        if let Err(e) = out.write_u8(len) { panic!("{:?}", e) };

        return Ok(());
    }

    Err(TypeMismatch { expected: String::from("u8"), got: String::from(args[0]) })
}

fn args_lbl(out: &mut Vec<u8>, args: &[&str], labels: &[(String, u16)]) -> Result<(), CompileError> {
    use CompileError::{SyntaxError, LabelNotFound};
    if args.len() != 1 { return Err(SyntaxError); }

    if let Some(ptr) = labels.iter().position(|x| x.0 == args[0]) {
        if let Err(e) = out.write_u16::<BigEndian>(labels[ptr].1) { panic!("{:?}", e) };
        return Ok(());
    }

    Err(LabelNotFound(String::from(args[0])))
}

fn args_cjm(out: &mut Vec<u8>, args: &[&str], labels: &[(String, u16)]) -> Result<(), CompileError> {
    use CompileError::{SyntaxError, LabelNotFound};
    if args.len() != 3 { return Err(SyntaxError); }

    if let Err(e) = dyn_args(2, 0, out, args) { return Err(e); }
    if let Some(ptr) = labels.iter().position(|x| x.0 == args[2]) {
        if let Err(e) = out.write_u16::<BigEndian>(labels[ptr].1) { panic!("{:?}", e) };
        return Ok(());
    }

    Err(LabelNotFound(String::from(args[2])))
}

/*******************************
 * Instructions implementation *
 *******************************/
fn impl_mkr(env: &mut ProgramEnvironment, mut args: Cursor<&mut [u8]>) -> Result<(), RuntimeError> {
    if env.len() >= 255 { return Err(RuntimeError::RingLimit); }
    env.mkring(args.read_u8().unwrap());
    Ok(())
}

fn impl_put(env: &mut ProgramEnvironment, mut args: Cursor<&mut [u8]>) -> Result<(), RuntimeError> {
    return match read_ring(env, &mut args) {
        Ok(ring) => {
            let value = args.read_u8().unwrap();
            env[ring][0] = value;

            Ok(())
        }
        Err(e) => Err(e)
    };
}

fn impl_rot(env: &mut ProgramEnvironment, mut args: Cursor<&mut [u8]>) -> Result<(), RuntimeError> {
    return match read_ring(env, &mut args) {
        Ok(ring) => {
            let value = args.read_u8().unwrap();
            env[ring].add_offset(value);

            Ok(())
        }
        Err(e) => Err(e)
    };
}

fn impl_swp(env: &mut ProgramEnvironment, mut args: Cursor<&mut [u8]>) -> Result<(), RuntimeError> {
    return match read_rings(2, env, &mut args) {
        Ok(rings) => {
            (env[rings[0]][0], env[rings[1]][0]) = (env[rings[1]][0], env[rings[0]][0]);

            Ok(())
        }
        Err(e) => Err(e)
    };
}

fn impl_inp(env: &mut ProgramEnvironment, mut args: Cursor<&mut [u8]>) -> Result<(), RuntimeError> {
    return match read_ring(env, &mut args) {
        Ok(ring) => {
            match (env.stdin)() {
                Ok(val) => {
                    env[ring][0] = val;
                    Ok(())
                }
                Err(e) => Err(StdinReadError(e))
            }
        }
        Err(e) => Err(e)
    };
}

fn impl_out(env: &mut ProgramEnvironment, mut args: Cursor<&mut [u8]>) -> Result<(), RuntimeError> {
    return match read_ring(env, &mut args) {
        Ok(ring) => {
            (env.stdout)(env[ring][0]);

            Ok(())
        }
        Err(e) => Err(e)
    };
}

fn impl_err(env: &mut ProgramEnvironment, mut args: Cursor<&mut [u8]>) -> Result<(), RuntimeError> {
    return match read_ring(env, &mut args) {
        Ok(ring) => {
            (env.stderr)(env[ring][0]);

            Ok(())
        }
        Err(e) => Err(e)
    };
}

fn impl_add(env: &mut ProgramEnvironment, mut args: Cursor<&mut [u8]>) -> Result<(), RuntimeError> {
    return match read_rings(3, env, &mut args) {
        Ok(rings) => {
            let value: u16 = env[rings[0]][0] as u16 + env[rings[1]][0] as u16;
            if value > 255 { return Err(RuntimeError::InvalidValue(value as isize)); }

            env[rings[2]][0] = value as u8;

            Ok(())
        }
        Err(e) => Err(e)
    };
}

fn impl_sub(env: &mut ProgramEnvironment, mut args: Cursor<&mut [u8]>) -> Result<(), RuntimeError> {
    return match read_rings(3, env, &mut args) {
        Ok(rings) => {
            let value: i16 = env[rings[0]][0] as i16 - env[rings[1]][0] as i16;
            if value < 0 { return Err(RuntimeError::InvalidValue(value as isize)); }

            env[rings[2]][0] = value as u8;

            Ok(())
        }
        Err(e) => Err(e)
    };
}

fn impl_mul(env: &mut ProgramEnvironment, mut args: Cursor<&mut [u8]>) -> Result<(), RuntimeError> {
    return match read_rings(3, env, &mut args) {
        Ok(rings) => {
            let value: u16 = env[rings[0]][0] as u16 * env[rings[1]][0] as u16;
            if value > 255 { return Err(RuntimeError::InvalidValue(value as isize)); }

            env[rings[2]][0] = value as u8;

            Ok(())
        }
        Err(e) => Err(e)
    };
}

fn impl_div(env: &mut ProgramEnvironment, mut args: Cursor<&mut [u8]>) -> Result<(), RuntimeError> {
    return match read_rings(3, env, &mut args) {
        Ok(rings) => {
            if env[rings[1]][0] == 0 { return Err(RuntimeError::DivideByZero); }

            env[rings[2]][0] = env[rings[0]][0] / env[rings[1]][0];

            Ok(())
        }
        Err(e) => Err(e)
    };
}

fn impl_jmp(env: &mut ProgramEnvironment, mut args: Cursor<&mut [u8]>) -> Result<(), RuntimeError> {
    env.mv_ip(args.read_u16::<BigEndian>().unwrap());
    Ok(())
}

fn impl_jeq(env: &mut ProgramEnvironment, mut args: Cursor<&mut [u8]>) -> Result<(), RuntimeError> {
    return match read_rings(2, env, &mut args) {
        Ok(rings) => {
            if env[rings[0]][0] != env[rings[1]][0] { return Ok(()); }

            env.mv_ip(args.read_u16::<BigEndian>().unwrap());

            Ok(())
        }
        Err(e) => Err(e)
    };
}

fn impl_jgt(env: &mut ProgramEnvironment, mut args: Cursor<&mut [u8]>) -> Result<(), RuntimeError> {
    return match read_rings(2, env, &mut args) {
        Ok(rings) => {
            if env[rings[0]][0] <= env[rings[1]][0] { return Ok(()); }

            env.mv_ip(args.read_u16::<BigEndian>().unwrap());

            Ok(())
        }
        Err(e) => Err(e)
    };
}

fn impl_jlt(env: &mut ProgramEnvironment, mut args: Cursor<&mut [u8]>) -> Result<(), RuntimeError> {
    return match read_rings(2, env, &mut args) {
        Ok(rings) => {
            if env[rings[0]][0] >= env[rings[1]][0] { return Ok(()); }

            env.mv_ip(args.read_u16::<BigEndian>().unwrap());

            Ok(())
        }
        Err(e) => Err(e)
    };
}

fn impl_hlt(env: &mut ProgramEnvironment, mut args: Cursor<&mut [u8]>) -> Result<(), RuntimeError> {
    let code = args.read_u8().unwrap();
    if code > 253 { println!("{}", env); }
    if code == 254 { return Ok(()); }
    Err(Halt(code))
}