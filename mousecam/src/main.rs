mod huffman;
use failure::Error;
use fftw::array::AlignedVec;
use fftw::plan::{C2CPlan, C2CPlan64};
use fftw::types::c64;
use huffman::CANON_HUFFMAN;
use image::{imageops, ImageBuffer};
use minifb::{Window, WindowOptions};
use rusb::{Context, UsbContext};
use std::convert::TryFrom;
use std::fs::File;
use std::sync::{atomic, atomic::Ordering::Relaxed, mpsc, Arc, RwLock};
use std::thread;
use std::time::Duration;
use uinput::event::controller::Controller::Mouse;
use uinput::event::controller::Mouse::Left;
use uinput::event::relative::Position::{X, Y};
use uinput::event::relative::Relative::Position;
use uinput::event::Event::{Controller, Relative};

const VID: u16 = 0x258a;
const NORM_PID: u16 = 0x0012;
const RECOV_PID: u16 = 0xff12;
// precalibrated brightness corrections for my sensor
const BRIGHTS: [f32; 900] = [
    0.99591297, 0.95660623, 0.94009526, 0.94225613, 0.89656039, 0.91175172, 0.91464702, 0.88420721,
    0.86930151, 0.90274682, 0.89936186, 0.87612322, 0.90178997, 0.90237748, 0.89794304, 0.95075035,
    0.94763121, 0.97187708, 0.97511597, 0.99489487, 1.01126693, 1.05061482, 1.04604104, 1.07790562,
    1.1071079, 1.14214142, 1.1597535, 1.1804973, 1.21775457, 1.30188448, 0.98821817, 0.93157564,
    0.92247877, 0.90790887, 0.89137936, 0.89539924, 0.86302362, 0.86432863, 0.86800373, 0.86405995,
    0.86429181, 0.86992016, 0.87346632, 0.87632754, 0.89209258, 0.90410634, 0.90879531, 0.94756926,
    0.94718002, 0.98228862, 0.97531751, 1.01290753, 1.02014112, 1.06797756, 1.06260304, 1.1019943,
    1.15297266, 1.17392731, 1.17236396, 1.23769055, 0.95911343, 0.92300538, 0.8913128, 0.86056588,
    0.86469142, 0.85268611, 0.835748, 0.83496048, 0.82968247, 0.85273269, 0.83495876, 0.84116473,
    0.84879654, 0.84644377, 0.87093388, 0.90429974, 0.89499049, 0.91211032, 0.92702394, 0.9468618,
    0.95434097, 0.99542197, 1.01420863, 1.00834224, 1.05104472, 1.08542138, 1.106646, 1.10634432,
    1.1787315, 1.21051537, 0.95516979, 0.88294718, 0.87256488, 0.84911087, 0.85850331, 0.85472799,
    0.84544395, 0.8242514, 0.82977068, 0.84371442, 0.84364777, 0.84377933, 0.83683197, 0.85954883,
    0.86262005, 0.89508326, 0.86947658, 0.90615736, 0.93448436, 0.94336661, 0.95079267, 0.98326915,
    1.00282077, 1.00235265, 1.02885413, 1.04310238, 1.1012766, 1.09459044, 1.14800217, 1.23864249,
    0.9305674, 0.88841917, 0.86670242, 0.84817211, 0.83523715, 0.85008377, 0.81964515, 0.81731436,
    0.82343861, 0.81085585, 0.83434423, 0.81473828, 0.82548368, 0.83612337, 0.85247655, 0.89132454,
    0.8892582, 0.89707169, 0.90636581, 0.93134047, 0.95376008, 0.97905157, 0.99000208, 0.99432204,
    1.03981507, 1.05999139, 1.08173838, 1.09351687, 1.137844, 1.20805427, 0.92239071, 0.87333099,
    0.8466539, 0.84536118, 0.82727241, 0.81216213, 0.82414762, 0.81941015, 0.82573898, 0.81270535,
    0.8253007, 0.81228242, 0.80809492, 0.82471035, 0.84152928, 0.85564345, 0.86102964, 0.88445001,
    0.92187527, 0.93933803, 0.94554697, 0.95276367, 0.97222399, 0.9921513, 1.01049894, 1.05910065,
    1.08945701, 1.10573241, 1.13188417, 1.18541092, 0.8954842, 0.87788007, 0.84058107, 0.85507556,
    0.83080517, 0.81537013, 0.81427892, 0.81304401, 0.80605654, 0.80631438, 0.79238168, 0.81112974,
    0.81703628, 0.82055664, 0.85227604, 0.86131105, 0.87515023, 0.90155759, 0.89686353, 0.92186271,
    0.94679332, 0.96475651, 0.97818788, 0.99025331, 1.02227654, 1.04938147, 1.07153335, 1.09980825,
    1.13461513, 1.15937911, 0.88628496, 0.87878111, 0.83908469, 0.83389506, 0.82730951, 0.82198095,
    0.81688498, 0.8136406, 0.81627541, 0.8115823, 0.79151933, 0.80279103, 0.81895047, 0.82822292,
    0.83314352, 0.87084417, 0.87161475, 0.89822526, 0.9131529, 0.90845354, 0.93825002, 0.95083723,
    0.97321019, 1.00669141, 1.03268168, 1.0449852, 1.07435298, 1.13740073, 1.13449143, 1.18142173,
    0.8820184, 0.87082735, 0.85202553, 0.82468857, 0.82245904, 0.81469085, 0.80999649, 0.81638378,
    0.80057551, 0.79869115, 0.80259099, 0.81006763, 0.8099092, 0.83709088, 0.83129359, 0.87874877,
    0.89456634, 0.91452336, 0.89960508, 0.92261508, 0.92226913, 0.94940694, 0.96962934, 0.99586409,
    1.01745567, 1.03230868, 1.06785954, 1.10549747, 1.14032499, 1.17261463, 0.90323304, 0.86014273,
    0.82854588, 0.85351461, 0.82972657, 0.81440474, 0.81047693, 0.80296575, 0.81037497, 0.80329952,
    0.821053, 0.80434395, 0.81271837, 0.82155829, 0.83877427, 0.86521312, 0.86158168, 0.8913128,
    0.90600766, 0.9186824, 0.93110115, 0.94510657, 0.9852648, 0.99603763, 1.02608375, 1.06687419,
    1.07045933, 1.09187226, 1.13794928, 1.18356137, 0.89086279, 0.88580714, 0.85991491, 0.83595114,
    0.81566675, 0.82480086, 0.82074914, 0.79636531, 0.82284258, 0.81598819, 0.81104869, 0.82567178,
    0.81665978, 0.83549507, 0.85456421, 0.86714317, 0.86678572, 0.90237949, 0.90489254, 0.91965044,
    0.93413806, 0.94322848, 0.979302, 0.98892373, 1.01939801, 1.07055534, 1.08266184, 1.13974214,
    1.13963012, 1.19572468, 0.88617465, 0.86922145, 0.84621079, 0.84691893, 0.83134297, 0.825044,
    0.80728476, 0.81247756, 0.8059669, 0.79618564, 0.81117513, 0.80740682, 0.82009567, 0.82289931,
    0.84810298, 0.8788934, 0.87567897, 0.90321294, 0.91882591, 0.92420771, 0.93758673, 0.97031324,
    0.98536049, 1.01267499, 1.02073896, 1.05836043, 1.07796574, 1.11594547, 1.16333063, 1.20081234,
    0.91177015, 0.87959059, 0.8473044, 0.82702121, 0.83813334, 0.81523909, 0.81381518, 0.81839887,
    0.81302283, 0.81061615, 0.79612629, 0.81572413, 0.81428055, 0.8346325, 0.84153626, 0.87767502,
    0.87193123, 0.89900126, 0.90789262, 0.920773, 0.95101994, 0.9946291, 0.99978208, 1.00884105,
    1.00723609, 1.07314276, 1.09454616, 1.11403701, 1.18298177, 1.2086875, 0.91547026, 0.87561662,
    0.86208316, 0.84808349, 0.83679918, 0.83209467, 0.83606824, 0.81494769, 0.8266826, 0.82714089,
    0.82798128, 0.8298097, 0.81181446, 0.83755906, 0.84419179, 0.89056759, 0.90599755, 0.90108517,
    0.9105979, 0.92314395, 0.94940472, 0.98051342, 1.00612238, 1.04716116, 1.02979399, 1.08120522,
    1.12195528, 1.12570224, 1.16715138, 1.22526211, 0.91075526, 0.88277048, 0.87787437, 0.85835623,
    0.85449403, 0.83603552, 0.84170032, 0.84895456, 0.833758, 0.83345493, 0.80936003, 0.82908414,
    0.83741734, 0.8541775, 0.85857777, 0.88885698, 0.89514447, 0.9172725, 0.92292351, 0.95859916,
    0.96412851, 0.99336069, 0.99694539, 1.00977734, 1.04663454, 1.1221197, 1.13577413, 1.10858371,
    1.15424828, 1.20587898, 0.92425191, 0.91033849, 0.8864437, 0.86665615, 0.86101319, 0.86307868,
    0.85200944, 0.84238694, 0.83201791, 0.84610494, 0.84955881, 0.84989684, 0.87236043, 0.86981575,
    0.88805563, 0.89210631, 0.8964574, 0.92596426, 0.92611217, 0.96293666, 0.96831987, 0.99220951,
    1.02249805, 1.03633136, 1.05670148, 1.09675282, 1.11824551, 1.15950498, 1.17452187, 1.24195926,
    0.96532791, 0.9233897, 0.89094884, 0.87934283, 0.85940867, 0.8818325, 0.86950266, 0.85064861,
    0.84388284, 0.86052026, 0.86743045, 0.86657102, 0.86813, 0.88013234, 0.8952116, 0.90212073,
    0.91336047, 0.95062564, 0.94752059, 0.93577937, 0.99314677, 0.9861722, 1.03245049, 1.0485166,
    1.07991342, 1.11491538, 1.14209964, 1.17552889, 1.20411152, 1.23904713, 0.95107789, 0.9162536,
    0.89989831, 0.88419758, 0.86115936, 0.85420806, 0.85931769, 0.86111733, 0.86408019, 0.8453876,
    0.8541002, 0.85608024, 0.86360763, 0.86978592, 0.87788007, 0.93356431, 0.92824314, 0.95579742,
    0.9685371, 0.99296938, 1.00004816, 1.04071101, 1.03263439, 1.08720968, 1.11959982, 1.1201808,
    1.13788228, 1.19252379, 1.25569719, 1.30447898, 0.97753286, 0.93900333, 0.90852676, 0.92637854,
    0.88900496, 0.86403236, 0.877694, 0.86301077, 0.8594924, 0.87781551, 0.85920853, 0.87288956,
    0.87589442, 0.89381175, 0.90013184, 0.93835199, 0.94944026, 0.95433199, 0.97968498, 1.00056099,
    1.01441905, 1.06246395, 1.07488794, 1.07060618, 1.09940603, 1.16990361, 1.1827818, 1.19533023,
    1.21368665, 1.34554508, 1.00055113, 0.96077552, 0.95326497, 0.94393491, 0.92444981, 0.89851361,
    0.89418199, 0.88758949, 0.88571628, 0.90991395, 0.87768072, 0.87029513, 0.88093283, 0.89485235,
    0.9206247, 0.97375427, 0.94371541, 0.98220779, 0.99697723, 1.0139603, 1.03938099, 1.06941282,
    1.09757148, 1.09690697, 1.15904137, 1.17831055, 1.19369885, 1.20289805, 1.2740755, 1.32274874,
    1.00349772, 0.97777544, 0.95543964, 0.96504327, 0.95220033, 0.92960607, 0.9110824, 0.8980563,
    0.88945309, 0.89652672, 0.90223705, 0.90180601, 0.92429611, 0.90749268, 0.9462988, 0.97877295,
    0.98098981, 1.00132136, 1.00329678, 1.06347739, 1.0618107, 1.07624766, 1.13299332, 1.121295,
    1.11398808, 1.16492012, 1.23864627, 1.22573209, 1.30107894, 1.34455988, 1.06666108, 0.99183607,
    0.99762669, 0.96047765, 0.96157228, 0.93872531, 0.95597304, 0.92616713, 0.94320656, 0.94354427,
    0.93758673, 0.95671448, 0.92907828, 0.92803936, 0.94876328, 1.00659653, 1.01632944, 1.00643926,
    1.04739898, 1.04278075, 1.06839086, 1.11473469, 1.14519677, 1.15597436, 1.19844003, 1.22249028,
    1.28849635, 1.34138233, 1.33952722, 1.43130109, 1.08238464, 1.09964732, 1.00549669, 1.03457715,
    0.99168823, 0.99067392, 0.96140371, 0.97708334, 0.96790418, 0.98982098, 0.95313288, 0.97031092,
    0.96659706, 1.00591041, 1.01806569, 1.03819776, 1.04254502, 1.06476367, 1.08789749, 1.08502962,
    1.131209, 1.14294235, 1.14441204, 1.18069993, 1.22636548, 1.2836668, 1.30447898, 1.32260649,
    1.37214062, 1.44078297, 1.12138175, 1.11809763, 1.07603934, 1.05562951, 1.02073383, 1.03864414,
    1.00491657, 0.98647184, 0.98174683, 0.99407362, 0.96706233, 0.98612667, 0.97723392, 0.99649008,
    1.01626073, 1.04629185, 1.07697596, 1.0875593, 1.09855489, 1.13749956, 1.15885603, 1.15218706,
    1.21192158, 1.2322601, 1.28211359, 1.30492359, 1.29829438, 1.36652696, 1.41577229, 1.49978951,
    1.18341642, 1.15243899, 1.10876846, 1.09675282, 1.04503633, 1.05686384, 1.04296031, 1.02547446,
    1.02137349, 1.03065965, 1.02206028, 1.01801461, 1.00830215, 1.05397362, 1.0461516, 1.10145893,
    1.11290971, 1.13230419, 1.16386776, 1.16568971, 1.19434524, 1.24204288, 1.28006328, 1.27233395,
    1.30991896, 1.38185866, 1.39339253, 1.38066456, 1.43217492, 1.49281623, 1.22145637, 1.2145584,
    1.17473946, 1.12233382, 1.1245668, 1.12718737, 1.10042257, 1.0841861, 1.07606502, 1.10030323,
    1.04537281, 1.06527514, 1.05299187, 1.09240423, 1.09727176, 1.13107659, 1.17588994, 1.16313724,
    1.18706832, 1.21864681, 1.2628006, 1.26715357, 1.28102089, 1.3288949, 1.3699035, 1.3956063,
    1.40708389, 1.4889058, 1.46898136, 1.50703527, 1.33330007, 1.28457695, 1.20924573, 1.16548216,
    1.15886596, 1.15725003, 1.14409587, 1.12851951, 1.09878391, 1.14085713, 1.12427084, 1.11494907,
    1.12233692, 1.15119675, 1.16069883, 1.1911832, 1.18710999, 1.21592665, 1.24618097, 1.30438674,
    1.31592059, 1.31522119, 1.3726047, 1.33447065, 1.37353846, 1.47268616, 1.49221245, 1.50418656,
    1.57464834, 1.62334965, 1.38743322, 1.33371195, 1.32784268, 1.24545052, 1.20986579, 1.21644419,
    1.22031778, 1.22446729, 1.18731142, 1.20538114, 1.17289591, 1.19216297, 1.15995239, 1.20009505,
    1.1681189, 1.24463312, 1.23461432, 1.29853117, 1.34148875, 1.2917733, 1.33227586, 1.39314859,
    1.438179, 1.43759314, 1.46110118, 1.52619115, 1.55096227, 1.58462723, 1.66221265, 1.68326797,
    1.50424789, 1.3980149, 1.39119532, 1.33058541, 1.2939067, 1.29815733, 1.2584345, 1.25884827,
    1.26507586, 1.23721512, 1.24252582, 1.24372912, 1.22859677, 1.24553461, 1.25382727, 1.31382037,
    1.31783066, 1.33195229, 1.34692053, 1.38080549, 1.41132637, 1.47326889, 1.48937572, 1.51348167,
    1.4503032, 1.50132638, 1.57654463, 1.63393769, 1.66880854, 1.83307004, 1.60213077, 1.54466354,
    1.52149927, 1.44226274, 1.44027164, 1.43941854, 1.39042793, 1.33101308, 1.33099125, 1.33763313,
    1.33839189, 1.36947359, 1.35197747, 1.31395649, 1.3558484, 1.40224155, 1.45237931, 1.39556311,
    1.45466992, 1.49090774, 1.49735472, 1.52371001, 1.5528435, 1.59569904, 1.60152382, 1.63168439,
    1.67411616, 1.69078576, 1.72364639, 1.90779813,
];

trait HIDReport {
    fn set_report(&self, iface: u8, buf: &[u8]) -> rusb::Result<usize>;
    fn get_report(&self, iface: u8, buf: &mut [u8]) -> rusb::Result<usize>;
}

impl<C: UsbContext> HIDReport for rusb::DeviceHandle<C> {
    fn set_report(&self, iface: u8, buf: &[u8]) -> rusb::Result<usize> {
        self.write_control(
            0x21,
            0x9,
            u16::from(buf[0]) | 3 << 8,
            u16::from(iface),
            buf,
            Duration::from_secs(5),
        )
    }
    fn get_report(&self, iface: u8, buf: &mut [u8]) -> rusb::Result<usize> {
        self.read_control(
            0xa1,
            0x1,
            u16::from(buf[0]) | 3 << 8,
            u16::from(iface),
            buf,
            Duration::from_secs(5),
        )
    }
}

struct SetReportArgs<'a> {
    mode: u8,
    args: [Option<u8>; 4],
    sendbuf: Option<&'a [u8]>,
}

#[allow(dead_code)]
impl<'a> SetReportArgs<'a> {
    fn new(mode: u8) -> Self {
        SetReportArgs {
            mode,
            args: [None, None, None, None],
            sendbuf: None,
        }
    }
    fn args1(self, arg1: u8) -> Self {
        Self {
            args: [Some(arg1), None, None, None],
            ..self
        }
    }
    fn args2(self, arg1: u8, arg2: u8) -> Self {
        Self {
            args: [Some(arg1), Some(arg2), None, None],
            ..self
        }
    }
    fn args3(self, arg1: u8, arg2: u8, arg3: u8) -> Self {
        Self {
            args: [Some(arg1), Some(arg2), Some(arg3), None],
            ..self
        }
    }
    fn args4(self, arg1: u8, arg2: u8, arg3: u8, arg4: u8) -> Self {
        Self {
            args: [Some(arg1), Some(arg2), Some(arg3), Some(arg4)],
            ..self
        }
    }
    fn set_sendbuf(self, buf: &'a [u8]) -> Self {
        SetReportArgs {
            sendbuf: Some(buf),
            ..self
        }
    }
    // converts the arguments to a buffer suitable for sending to the device
    fn to_buf(&self) -> Vec<u8> {
        let mut ret_buf = Vec::new();
        ret_buf.push(2u8);
        ret_buf.push(self.mode);
        ret_buf.extend(self.args.iter().map(|x| x.unwrap_or(0)));
        ret_buf.extend_from_slice(&[0, 0]);
        if let Some(buf) = self.sendbuf {
            ret_buf.extend(buf.iter());
            while ret_buf.len() % 8 != 0 {
                ret_buf.push(0);
            }
        } else {
            ret_buf.extend_from_slice(&[8; 0]);
        }
        ret_buf
    }
}

struct RecHandle<C: rusb::UsbContext>(rusb::DeviceHandle<C>);

impl<C: rusb::UsbContext> RecHandle<C> {
    fn new(context: &C) -> rusb::Result<Self> {
        for device in context.devices()?.iter() {
            let device_desc = device.device_descriptor()?;
            if device_desc.vendor_id() == VID && device_desc.product_id() == NORM_PID {
                let mut handle = device.open()?;
                handle.detach_kernel_driver(1)?;
                handle.claim_interface(1)?;
                let ret =
                    handle.set_report(1, &[2, 3, 0xaa, 0xbb, 9, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
                match ret {
                    // Err(rusb::Error::Pipe) (or Io, depending on a hub being inbetween?) is actually expected here
                    Err(rusb::Error::Pipe) | Err(rusb::Error::Io) => (),
                    Ok(_) => {
                        eprintln!("Error: unexpected non-error");
                        std::process::exit(1);
                    }
                    _ => {
                        ret?;
                    }
                };
                thread::sleep(Duration::from_secs(5));
                break;
            }
        }
        loop {
            for device in context.devices()?.iter() {
                let device_desc = device.device_descriptor()?;
                if device_desc.vendor_id() == VID && device_desc.product_id() == RECOV_PID {
                    let mut handle = device.open()?;
                    handle.claim_interface(0)?;
                    return Ok(RecHandle(handle));
                }
            }
        }
    }

    fn send_report(&self, args: SetReportArgs) -> Result<(), Error> {
        self.0.set_report(0, &args.to_buf())?;
        Ok(())
    }

    fn recv_report(&self, len: usize) -> Result<Vec<u8>, Error> {
        let mut recv_buf = vec![0; len];
        recv_buf[0] = 5;
        self.0.get_report(0, &mut recv_buf)?;
        Ok(recv_buf)
    }

    /// uploads the laser rom to 0xc006 by writing the registers of the adns-9800
    /// in the following manner: 
    /// 0x30 <- 0x44
    /// 0x31 <- 0x00 (arg2)
    /// 0x32 <- 0x00 (arg3)
    /// repeated 0x35 <- [bytes from file]
    /// 
    /// note that 0x35 is an auto-incrementing register, so we don't need to write
    /// new addresses to 0x31 and 0x32 which are set to write to 0xc006 initially
    /// 
    /// all this is done on the mouse µC itself because of timing constraints
    fn upload_laser<T: std::io::Read>(&self, file: &mut T) -> Result<(), Error> {
        let mut read_vec: Vec<u8> = Vec::new();
        file.read_to_end(&mut read_vec)?;
        while read_vec.len() % 8 != 0 {
            read_vec.push(0);
        }
        let len8th = u8::try_from(read_vec.len() / 8)?;
        self.send_report(
            SetReportArgs::new(15)
                .args4(len8th, 0x35, 0x00, 0x00)
                .set_sendbuf(&read_vec),
        )
    }

    fn write_port(&self, port: u8, val: u8) -> Result<(), Error> {
        self.send_report(SetReportArgs::new(7).args2(port, val))?;
        self.recv_report(8)?;
        Ok(())
    }

    /// this reads 2040 bytes from the pixel burst register (0x64)
    /// note that the first 8 bytes are the header
    fn speed_burst(&self) -> Result<Vec<u8>, Error> {
        self.send_report(SetReportArgs::new(12))?;
        self.recv_report(2048)
    }
}

impl<C: rusb::UsbContext> Drop for RecHandle<C> {
    fn drop(&mut self) {
        let _ = self.send_report(SetReportArgs::new(1));
    }
}

/// the format of a compressed frame is as follos:
/// 16 bits: shutter value, uncompressed
/// 8 bits: first pixel as-is, uncompressed
/// for other pixels, differences are calculated like this:
/// pixels at (0, x): (img[0, x - 1] - img[0, x]) % 255
/// pixels at (y, 0): (img[y - 1, 0] - img[y, 0]) % 255
/// pixels at (y, x): ((img[y - 1, x] + img[y, x - 1]) / 2 - img[y, x]) % 255
/// and then they are encoded according to the huffman table in huffman.rs
fn decode(
    rx: mpsc::Receiver<Vec<u8>>,
    tx: mpsc::Sender<(u16, [[u8; 30]; 30])>,
    running: Arc<atomic::AtomicBool>,
    fps: Arc<atomic::AtomicU32>,
) {
    let mut x = 0;
    let mut y = 0;
    let mut cur_frame = [[0; 30]; 30];
    let mut first = true;
    let mut cur_code = 0u64;
    let mut cur_len = 0;
    let mut cur_len_index = 0;
    let mut shutter = None;
    for v in rx.iter() {
        if !running.load(Relaxed) {
            return;
        }
        //        println!("{:?}", v);
        for cur_byte in &v[8..] {
            if first {
                // the very first byte does not contain anything
                first = false;
                continue;
            }
            for bit in 0..8 {
                cur_len += 1;
                cur_code <<= 1;
                cur_code |= u64::from(cur_byte >> (7 - bit) & 1u8);
                let (clen, cmin, cmax, carr) = &CANON_HUFFMAN[cur_len_index];
                let k;
                // first 16 bits are the shutter value
                if shutter.is_none() {
                    if cur_len == 16 {
                        shutter = Some(cur_code as u16);
                        cur_code = 0;
                        cur_len = 0;
                        cur_len_index = 0;
                    }
                    continue;
                } else if x == 0 && y == 0 {
                    // first pixel is the first byte
                    if cur_len == 8 {
                        k = cur_code as u8;
                        cur_code = 0;
                        cur_len = 0;
                        cur_len_index = 0;
                    } else {
                        continue;
                    }
                } else if cur_len == *clen {
                    // otherwise it is huffman encoded
                    if cur_code <= *cmax {
                        k = carr[(cur_code - cmin) as usize];
                        cur_code = 0;
                        cur_len = 0;
                        cur_len_index = 0;
                    } else {
                        cur_len_index += 1;
                        continue;
                    }
                } else {
                    continue;
                }
                cur_frame[y][x] = match (x, y) {
                    // first pixel encoded as-is
                    (0, 0) => k,
                    // other pixels encoded as difference
                    (0, _) => cur_frame[y - 1][x].wrapping_sub(k),
                    (_, 0) => cur_frame[y][x - 1].wrapping_sub(k),
                    _ => (((u16::from(cur_frame[y - 1][x]) + u16::from(cur_frame[y][x - 1])) / 2)
                        as u8)
                        .wrapping_sub(k),
                };
                // move a scanline forward if we have reached the end of the line
                x += 1;
                if x == 30 {
                    x = 0;
                    y += 1;
                }
                // if we have reached the end of the frame, send it to the processor thread
                if y == 30 {
                    y = 0;
                    // write image
                    let res = tx.send((shutter.unwrap_or(128), cur_frame));
                    if res.is_err() {
                        running.store(false, Relaxed);
                        return;
                    }
                    fps.fetch_add(1, Relaxed);
                    shutter = None;
                }
            }
        }
    }
    running.store(false, Relaxed);
}

// cubic interpolation matrix
const CUBIC_INTERPOL: [[f64;16];16] = [[1./4., -3./4., 3./4., -1./4., -3./4., 9./4., -9./4., 3./4., 3./4., -9./4., 9./4., -3./4., -1./4., 3./4., -3./4., 1./4.],
 [-1./2., 5./4., -1., 1./4., 3./2., -15./4., 3., -3./4., -3./2., 15./4., -3., 3./4., 1./2., -5./4., 1., -1./4.],
 [1./4., 0., -1./4., 0., -3./4., 0., 3./4., 0., 3./4., 0., -3./4., 0., -1./4., 0., 1./4., 0.],
 [0., -1./2., 0., 0., 0., 3./2., 0., 0., 0., -3./2., 0., 0., 0., 1./2., 0., 0.],
 [-1./2., 3./2., -3./2., 1./2., 5./4., -15./4., 15./4., -5./4., -1., 3., -3., 1., 1./4., -3./4., 3./4., -1./4.],
 [1., -5./2., 2., -1./2., -5./2., 25./4., -5., 5./4., 2., -5., 4., -1., -1./2., 5./4., -1., 1./4.],
 [-1./2., 0., 1./2., 0., 5./4., 0., -5./4., 0., -1., 0., 1., 0., 1./4., 0., -1./4., 0.],
 [0., 1., 0., 0., 0., -5./2., 0., 0., 0., 2., 0., 0., 0., -1./2., 0., 0.],
 [1./4., -3./4., 3./4., -1./4., 0., 0., 0., 0., -1./4., 3./4., -3./4., 1./4., 0., 0., 0., 0.],
 [-1./2., 5./4., -1., 1./4., 0., 0., 0., 0., 1./2., -5./4., 1., -1./4., 0., 0., 0., 0.],
 [1./4., 0., -1./4., 0., 0., 0., 0., 0., -1./4., 0., 1./4., 0., 0., 0., 0., 0.],
 [0., -1./2., 0., 0., 0., 0., 0., 0., 0., 1./2., 0., 0., 0., 0., 0., 0.],
 [0., 0., 0., 0., -1./2., 3./2., -3./2., 1./2., 0., 0., 0., 0., 0., 0., 0., 0.],
 [0., 0., 0., 0., 1., -5./2., 2., -1./2., 0., 0., 0., 0., 0., 0., 0., 0.],
 [0., 0., 0., 0., -1./2., 0., 1./2., 0., 0., 0., 0., 0., 0., 0., 0., 0.],
 [0., 0., 0., 0., 0., 1., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0.]];


fn cross(v: &[f64; 16], w: &[f64; 16]) -> f64 {
    v.iter().zip(w).map(|(x, y)| x * y).sum()
}

// get the coefficients of the cubic bivariate polynomial
// interpolated from the 16 surrounding pixels
fn get_parameters(pixels: &[f64; 16]) -> [f64; 16] {
    let mut ret = [0.0f64; 16];
    for (i, x) in CUBIC_INTERPOL.iter().enumerate() {
        ret[i] = cross(pixels, x);
    }
    ret
}

// interpolate a pixel at a given offset using bicubic interpolation
fn bicubic_pix(pixels: [f64; 16], y: f64, x: f64) -> f64 {
    let par = get_parameters(&pixels);
    let mut val = 0.0f64;
    for x_idx in 0..4 {
        let mut ypol = 0.0f64;
        for y_idx in 0..4 {
            ypol *= y;
            ypol += par[4*x_idx + y_idx];
        }
        val *= x;
        val += ypol;
    }
    val
}

// bilinear interpolation, unused but left here in case i need it later
// 0 <= x, y <= 1
#[allow(unused)]
fn bilinear_pix(pixels: [[f64; 2]; 2], x: f64, y: f64) -> (f64, f64, f64) {
    let x0diff = pixels[0][1] - pixels[0][0];
    let x1diff = pixels[1][1] - pixels[1][0];
    let y0diff = pixels[1][0] - pixels[0][0];
    let y1diff = pixels[1][1] - pixels[0][1];
    let xdiff = (1. - y) * x0diff + y * x1diff;
    let ydiff = (1. - x) * y0diff + x * y1diff;
    let xyval = pixels[0][0] + x * x0diff + y * ydiff;
    (xyval, xdiff, ydiff)
}

// calculate the mean-squared error between two images with a given offset
// from each other
fn get_mean_square_error(
    old_image: &[[f64; 30]; 30],
    image: &[[f64; 30]; 30],
    xoff: f64,
    yoff: f64,
) -> f64 {
    let xoff = -xoff;
    let yoff = -yoff;
    let x_floor = xoff.floor();
    let x_remain = xoff - x_floor;
    let y_floor = yoff.floor();
    let y_remain = yoff - y_floor;
    if xoff >= 25.0 || yoff >= 25.0 || xoff <= -25.0 || yoff <= -25.0 {
        panic!("Out of range!");
    }
    let x_base = 0.max(x_floor as isize) as usize;
    let y_base = 0.max(y_floor as isize) as usize;
    let old_x_base = 0.max(-x_floor as isize) as usize;
    let old_y_base = 0.max(-y_floor as isize) as usize;
    let x_size = 28 - x_floor.abs() as usize;
    let y_size = 28 - y_floor.abs() as usize;
    let mut square_err: f64 = 0.;
    for y in 1..y_size {
        for x in 1..x_size {
            let nx = x_base + x;
            let ny = y_base + y;
            let val = bicubic_pix(
                [
                        image[ny - 1][nx - 1],
                        image[ny - 1][nx],
                        image[ny - 1][nx + 1],
                        image[ny - 1][nx + 2],
                        image[ny][nx - 1],
                        image[ny][nx],
                        image[ny][nx + 1],
                        image[ny][nx + 2],
                        image[ny + 1][nx - 1],
                        image[ny + 1][nx],
                        image[ny + 1][nx + 1],
                        image[ny + 1][nx + 2],
                        image[ny + 2][nx - 1],
                        image[ny + 2][nx],
                        image[ny + 2][nx + 1],
                        image[ny + 2][nx + 2],
                ],
                x_remain,
                y_remain,
            );
            let tdiff = val - old_image[old_y_base + y][old_x_base + x];
            square_err += tdiff * tdiff;
        }
    }
    square_err/((y_size as f64 - 1.)*(x_size as f64 - 1.))
}

// after a rough whole-pixel alignment, do a subpixel alignment using using cubic interpolation
fn look_around_you (
    old_image: &[[f64; 30]; 30],
    image: &[[f64; 30]; 30],
    init_x: f64,
    init_y: f64,
    max_iter: u64,
) -> (f64, f64) {
    let mut x_est = init_x;
    let mut y_est = init_y;
    let mut best_error = get_mean_square_error(old_image, image, x_est, y_est);
    let mut rad = 0.5f64;
    let rad_num = 2;
    // we repeatedly search 25 surrounding points and reduce the radius after each loop
    // each time choosing the point with the lowest error
    for _ in 0..max_iter {
        let mut new_x = x_est;
        let mut new_y = y_est;
        for x in (-rad_num)..=rad_num {
            let x_frac = x as f64/rad_num as f64*rad;
            for y in (-rad_num)..=rad_num {
                let y_frac = y as f64/rad_num as f64*rad;
                if x == 0 && y == 0 {
                    continue;
                }
                let new_err = get_mean_square_error(old_image, image, x_est + x_frac,y_est + y_frac);
                if new_err < best_error {
                    new_x = x_est + x_frac;
                    new_y = y_est + y_frac;
                    best_error = new_err;
                }
            }
        }
        x_est = new_x;
        y_est = new_y;
        rad /= rad_num as f64*2.;
    }
    (x_est, y_est)
}

// the image processing pipeline
fn img_proc(rx: mpsc::Receiver<(u16, [[u8; 30]; 30])>, imgbuf: Arc<RwLock<[f64; 3600]>>) {
    let mut planfw: C2CPlan64 = C2CPlan::aligned(
        &[60, 60],
        fftw::types::Sign::Forward,
        fftw::types::Flag::MEASURE,
    )
    .unwrap();
    let mut planinv: C2CPlan64 = C2CPlan::aligned(
        &[60, 60],
        fftw::types::Sign::Backward,
        fftw::types::Flag::MEASURE,
    )
    .unwrap();
    let mut a = AlignedVec::new(3600);
    for y in 0..30 {
        for x in 0..30 {
            a[x + 60 * y] = c64::new(1.0f64, 0.0);
            a[x + 30 + 60 * y] = c64::new(0.0, 0.0);
            a[x + 60 * (y + 30)] = c64::new(0.0, 0.0);
            a[x + 30 + 60 * (y + 30)] = c64::new(0.0, 0.0);
        }
    }
    let mut fweight = AlignedVec::new(3600);
    planfw.c2c(&mut a, &mut fweight).unwrap();
    let mut fa = AlignedVec::new(3600);
    let mut fa_old = AlignedVec::<c64>::new(3600);
    let mut b = AlignedVec::new(3600);
    let mut fws = AlignedVec::<c64>::new(3600);
    let mut fres = AlignedVec::new(3600);
    let mut prev_frame = Box::new([[0.0f64; 30]; 30]);
    let mut counter = 1;
    let mut xdiff = 0.0;
    let mut ydiff = 0.0;
    let mut first = true;
    // controller for the cursor on the host
    let mut dev = uinput::default()
        .unwrap()
        .name("crazy_mouse")
        .unwrap()
        .event(Controller(Mouse(Left)))
        .unwrap()
        .event(Relative(Position(X)))
        .unwrap()
        .event(Relative(Position(Y)))
        .unwrap()
        .create()
        .unwrap();
    for (shutter, buf) in rx {
        let mut new_frame = Box::new([[0.0f64; 30]; 30]);
        for (y, row) in buf.iter().enumerate() {
            for (x, pix) in row.iter().enumerate() {
                // make a 60x60 and only fill a 30x30 region
                // also do brightness corretion
                new_frame[y][x] =
                    f64::from(BRIGHTS[x + 30 * y]) * f64::from(*pix) / f64::from(shutter);
                a[x + 60 * y] = fftw::types::c64::new(new_frame[y][x], 0.0);
                b[x + 60 * y] = a[x + 60 * y] * a[x + 60 * y];

                // everything else is 0
                a[x + 30 + 60 * y] = c64::new(0.0, 0.0);
                b[x + 30 + 60 * y] = c64::new(0.0, 0.0);

                a[x + 60 * (y + 30)] = c64::new(0.0, 0.0);
                b[x + 60 * (y + 30)] = c64::new(0.0, 0.0);

                a[x + 30 + 60 * (y + 30)] = c64::new(0.0, 0.0);
                b[x + 30 + 60 * (y + 30)] = c64::new(0.0, 0.0);
            }
        }
        planfw.c2c(&mut a, &mut fa).unwrap();
        if !first {
            for (res, (x, (y, ws))) in fres
                .iter_mut()
                .zip(fa_old.iter().zip(fa.iter().zip(fws.iter())))
            {
                *res = ws.conj() - 2.0 * x * y.conj();
            }
        }
        for (old, new) in fa_old.iter_mut().zip(fa.iter()) {
            *old = *new;
        }
        planfw.c2c(&mut b, &mut fws).unwrap();
        for (s, w) in fws.iter_mut().zip(fweight.iter()) {
            *s = w * s.conj();
        }
        if !first {
            for (res, ws) in fres.iter_mut().zip(fws.iter()) {
                *res += ws;
            }
            planinv.c2c(&mut fres, &mut b).unwrap();
            for y in 0..60 {
                for x in 0..60 {
                    let xrel = (30 - x as isize).abs();
                    let yrel = (30 - y as isize).abs();
                    let area = (xrel * yrel) as f64;
                    b[x + 60 * y] = if area == 0.0 {
                        c64::new(std::f64::INFINITY, 0.0)
                    } else {
                        b[x + 60 * y] / area
                    };
                }
            }
            let mut buf = imgbuf.write().unwrap();
            for y in 0..60 {
                let y_shift = (90 - y) % 60;
                for x in 0..60 {
                    let x_shift = (30 + x) % 60;
                    buf[x + 60 * y] = a[x_shift + 60 * y_shift].re * 256.0;
                }
            }
            let mut cur_min = std::f64::INFINITY;
            let mut min_x: i32 = 0;
            let mut min_y: i32 = 0;
            for y_neg in -10..=10 {
                let y = ((60 + y_neg) % 60) as usize;
                for x_neg in -10..=10 {
                    let x = ((60 + x_neg) % 60) as usize;
                    let val = b[x + 60 * y].re;
                    if val < cur_min {
                        cur_min = val;
                        min_x = x_neg;
                        min_y = y_neg;
                    }
                }
            }
            // refine the solution we got through fft to an sub-pixel offset
            let (new_x, new_y) =
                look_around_you(&prev_frame, &new_frame, min_x as f64, min_y as f64, 2);
            std::mem::swap(&mut prev_frame, &mut new_frame);
            xdiff += new_y;
            ydiff -= new_x;
            counter -= 1;
            if counter == 0 {
//                println!("{} {}", xdiff, ydiff);
                let dx_round = xdiff.round();
                let dy_round = ydiff.round();
                dev.send(X, dx_round as i32).unwrap();
                dev.send(Y, dy_round as i32).unwrap();
                dev.synchronize().unwrap();
                xdiff -= dx_round;
                ydiff -= dy_round;
                counter = 5;
            }
        } else {
            first = false;
        }
    }
}

// display the image on the host
fn display(imgbuf: Arc<RwLock<[f64; 3600]>>, running: Arc<atomic::AtomicBool>) {
    let mut window = Window::new(
        "mousecam",
        600,
        600,
        WindowOptions {
            resize: true,
            ..WindowOptions::default()
        },
    )
    .expect("Could not open window buffer");
    window.limit_update_rate(Some(Duration::from_micros(16600)));
    while window.is_open() && !window.is_key_down(minifb::Key::Escape) {
        if !running.load(Relaxed) {
            return;
        }
        let img;
        let buf = imgbuf.read().unwrap();
        img = ImageBuffer::from_fn(60, 60, |x, y| {
            image::Luma([(buf[60 * x as usize + y as usize] * 0.2)
                .round()
                .min(255.0)
                .max(0.0) as u8])
        });
        drop(buf);
        let (x, y) = window.get_size();
        let scale_img = imageops::resize(&img, x as u32, y as u32, imageops::FilterType::Nearest);
        let mut scale_buf = Vec::with_capacity(x * y);
        for pix in scale_img.pixels() {
            scale_buf.push(u32::from(pix[0]) * 0x10101);
        }
        window.update_with_buffer(&scale_buf, x, y).unwrap();
    }
    running.store(false, Relaxed);
}

// get a frame from the device and send it to the decompressor, hoping that the order is preserved
fn frameget<C: rusb::UsbContext>(recdev: Arc<RecHandle<C>>, tx: mpsc::Sender<Vec<u8>>) {
    let mut failcount = 0;
    loop {
        let retres = recdev.speed_burst();
        match retres {
            Ok(ret) => {
                failcount = 0;
                let suc = tx.send(ret);
                if let Err(_) = suc {
                    return;
                }
            }
            Err(err) => {
                failcount += 1;
                if failcount > 3 {
                    eprintln!("Thread got error {}", err);
                    return;
                }
            }
        }
    }
}

fn main() -> Result<(), Error> {
    let context = Context::new()?;
    let recdev = RecHandle::new(&context)?;
    let mut file = File::open("ang/firmware")?;
    recdev.upload_laser(&mut file)?; // load rom at 0xc006
    // note that the rom is loaded first at 0xc006, but that is just the loader stage
    // which loads itself into 0x8000 on the device because 0xc006 is also used as the
    // framebuffer
    recdev.write_port(0x30, 0x44)?; // enable debug
    recdev.write_port(0x31, 0)?; // set lower addr byte to 0
    recdev.write_port(0x32, 0xc0)?; // set upper addr byte to 0xC0
                                    // address is now 0xC000
    recdev.write_port(0x36, 0x02)?; // write byte 0x02 (ljmp opcode)
    recdev.write_port(0x31, 1)?; // increase address to 0xC001
    recdev.write_port(0x36, 0xc0)?; // write byte 0xc0 (upper byte jump target)
    recdev.write_port(0x31, 2)?; // increase address to 0xC002
    recdev.write_port(0x36, 0x06)?; // write byte 0x06 (lower byte jump target)
    recdev.write_port(0x46, 0x00)?; // executes code at 0xc006
    thread::sleep(Duration::from_millis(10));
    recdev.write_port(0, 0)?; // just reset the addresses
    let recref = Arc::new(recdev);
    let (tx, rx) = mpsc::channel();
    let (tx2, rx2) = mpsc::channel();
    let frame = Arc::new(RwLock::new([0f64; 3600]));
    let frame_win = frame.clone();
    let running = Arc::new(atomic::AtomicBool::new(true));
    let run2 = running.clone();
    let run3 = running.clone();
    let fps = Arc::new(atomic::AtomicU32::new(0));
    let fpsclone = fps.clone();
    let mut handles = Vec::new();
    for _ in 0..3 {
        let txcl = tx.clone();
        let recrefcl = recref.clone();
        handles.push(thread::spawn(move || frameget(recrefcl, txcl)));
    }
    handles.push(thread::spawn(move || display(frame_win, run2)));
    handles.push(thread::spawn(move || {
        while run3.load(Relaxed) {
            thread::sleep(Duration::from_secs(1));
            let fpscount = fpsclone.swap(0, Relaxed);
            println!("{}", fpscount);
        }
        run3.store(false, Relaxed);
    }));
    handles.push(thread::spawn(move || img_proc(rx2, frame)));
    decode(rx, tx2, running, fps);
    for handle in handles {
        handle.join().unwrap();
    }
    Ok(())
}
