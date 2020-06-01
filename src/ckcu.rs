/// Clock Control Unit
use crate::ht32::{CKCU, FMC};
use crate::time::{Hertz, U32Ext};

/// Extension trait that constrains the `Ckcu` peripheral
pub trait CkcuExt {
    /// Constrains the `Ckcu` peripheral so it plays nicely with the other abstractions
    fn constrain(self) -> Ckcu;
}

impl CkcuExt for CKCU {
    fn constrain(self) -> Ckcu {
        Ckcu {
            configuration: Configuration {
                ckout: None,
                hse: None,
                lse: None,
                ck_usb: None,
                ck_adc_ip: None,
                hclk: None,
                ck_sys: None,
            },
        }
    }
}

/// Constrained Ckcu peripheral
pub struct Ckcu {
    pub configuration: Configuration,
}

/// High Speed Internal Oscillator at 8 Mhz
const HSI: u32 = 8_000_000;
/// Low Speed Internal Oscillator at 32 Khz
const LSI: u32 = 32_000;

/// All clocks that can be outputted via CKOUT.
/// See User Manual page 91.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CkoutSrc {
    /// Output the CK_REF, no prescaler
    CkRef,
    /// Output the HCLK, divided by 16
    Hclk,
    /// Output the CK_SYS, divided by 16
    CkSys,
    /// Output the CK_HSE, divided by 16
    CkHse,
    /// Output the CK_HSI, divided by 16
    CkHsi,
    /// Output the CK_LSE, no prescaler
    CkLse,
    /// Output the CK_LSI, no prescaler
    CkLsi,
}

/// Representation of the HT32F52342 clock tree.
///
/// Note that this struct only represents the targeted values.
/// As there are constrains as to which clock values can be achieved,
/// these values will probably never be achieved to 100% correctness.
/// See User Manual page 83
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Configuration {
    /// Which clock should be outputted via CKOUT
    ckout: Option<CkoutSrc>,
    /// The frequency of an HSE, should one be given
    hse: Option<Hertz>,
    /// The frequency of an LSI, should one be given.
    lse: Option<Hertz>,
    /// The optimal frequency for CK_USB, aka the USB clock
    ck_usb: Option<Hertz>,
    /// The optimal frequency for CK_ADC_IP, aka the ADC clock
    ck_adc_ip: Option<Hertz>,
    /// The optimal frequency for CK_SYS
    ck_sys: Option<Hertz>,
    /// The optimal frequency for HCLK, aka the AHB bus
    hclk: Option<Hertz>,
}

/// Frozen core clock frequencies
///
/// The existence of this value indicates that the core clock
/// configuration can no longer be changed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Clocks {
    /// Which clock should be outputted via CKOUT, if any
    ckout: Option<CkoutSrc>,
    /// The frequency for CK_USB, aka the USB clock
    ck_usb: Hertz,
    /// The frequency for CK_ADC_IP, aka the ADC clock
    ck_adc_ip: Hertz,
    /// The frequency for CK_SYS
    ck_sys: Hertz,
    /// The frequency for STCLK, aka the SysTick clock
    stclk: Hertz,
    /// The frequency for HCLK, aka the AHB bus
    hclk: Hertz,
}

impl Configuration {
    /// Set the clock that should be outputted via CKOUT
    pub fn ckout(mut self, ckout: CkoutSrc) -> Self {
        self.ckout = Some(ckout);
        self
    }

    /// Notifies the Configuration mechanism that an HSE is in use, this
    /// will make it prefer the HSE over the HSI in case the HSI should
    /// turn out to be the fitting clock for a certain part of the
    /// configuration.
    pub fn use_hse<F>(mut self, hse: F) -> Self
    where
        F: Into<Hertz>,
    {
        self.hse = Some(hse.into());
        self
    }

    /// Notifies the Configuration mechanism that an LSI is in use, this
    /// will make it prefer the LSE over the LSI in case the LSI should
    /// turn out to be the fitting clock for a certain part of the
    /// configuration.
    pub fn use_lse<F>(mut self, freq: F) -> Self
    where
        F: Into<Hertz>,
    {
        self.lse = Some(freq.into());
        self
    }

    /// Sets the desired value for CK_USB
    pub fn ck_usb<F>(mut self, freq: F) -> Self
    where
        F: Into<Hertz>,
    {
        self.ck_usb = Some(freq.into());
        self
    }

    /// Sets the desired value for CK_ADC_IP
    pub fn ck_adc_ip<F>(mut self, freq: F) -> Self
    where
        F: Into<Hertz>,
    {
        self.ck_adc_ip = Some(freq.into());
        self
    }

    /// Sets the desired value for CK_SYS
    pub fn ck_sys<F>(mut self, freq: F) -> Self
    where
        F: Into<Hertz>,
    {
        self.ck_sys = Some(freq.into());
        self
    }

    /// Sets the desired value for HCLK
    pub fn hclk<F>(mut self, freq: F) -> Self
    where
        F: Into<Hertz>,
    {
        self.hclk = Some(freq.into());
        self
    }

    /// Freeze the configuration into a Clocks struct and apply it
    pub fn freeze(self) -> Clocks {
        // High speed oscillator
        let hso = self.hse.unwrap_or(HSI.hz());
        // PLL source clock, see top left corner of the clock tree,
        // User manual page 83
        let pllsrc = self.hse.is_some();

        let mut pll_target_clock = None;

        // Refer to User manual page 96 for SW values
        let (sw, mut ck_sys) = match self.ck_sys {
            Some(ck_sys) => {
                // Maximum frequency for CK_SYS is 48 Mhz
                // Refer to User Manual page 83 at the CK_SYS mux
                assert!(ck_sys <= 48.mhz().into());

                if self.lse.map(|l| l == ck_sys).unwrap_or(false) {
                    (0b110, self.lse.unwrap())
                } else if self.hse.map(|h| h == ck_sys).unwrap_or(false) {
                    (0b010, self.hse.unwrap())
                } else if ck_sys.0 == LSI {
                    (0b111, LSI.hz())
                } else if ck_sys.0 == HSI {
                    (0b011, HSI.hz())
                }
                // If no exact match is found, use the pll
                else {
                    pll_target_clock = Some(ck_sys);
                    (0b000, ck_sys)
                }
            }
            // If no value is given select the low speed oscillator,
            // furthermore automatically choose LSE if it's provided.
            None => match self.lse {
                Some(lse) => (0b110, lse),
                None => (0b111, LSI.hz()),
            },
        };

        let mut ck_usb = match self.ck_usb {
            Some(ck_usb) => {
                // Maximum frequency for CK_USB is 48 Mhz
                // Refer to User Manual page 83, top right corner
                assert!(ck_usb < 48.mhz().into());
                if pll_target_clock.is_none() {
                    pll_target_clock = self.ck_usb;
                }
                ck_usb
            }
            None => match pll_target_clock {
                Some(clock) => clock,
                None => 0.hz(),
            },
        };

        let (mut nf2, mut no2) = (None, None);
        if pll_target_clock.is_some() {
            // According to User Manual page 87
            // pll_out = CK_in (NF2/NO2)
            let optimal_divider = pll_target_clock.unwrap().0 as f32 / hso.0 as f32;
            let mut closest = (1, 1);
            let mut difference = f32::MAX;

            // Try all combinations of NF2 and NO2, there are only
            // 64 so this should be fine.
            for nf2 in 1..17 {
                // According to User Manual page 87
                // VCO_out = CK_in * (NF1*NF2)/2 = CK_in * (4*NF2)/2
                // and VCO_out must be between 48 and 96 Mhz
                let vco_out = hso.0 * (4 * nf2) / 2;
                if vco_out >= 48_000_000 && vco_out <= 96_000_000 {
                    for no2 in &[1, 2, 4, 8] {
                        let current_divider = nf2 as f32 / *no2 as f32;

                        // According to User Manual page 87
                        // The maximum output frequency for the PLL must be
                        // bettween 4 and 48 Mhz
                        let current_output = current_divider * hso.0 as f32;
                        if !(current_output > 4_000_000.0 && current_output < 48_000_000.0) {
                            continue;
                        }

                        let mut current_difference = optimal_divider - current_divider;
                        if current_difference < 0.0 {
                            current_difference *= -1.0
                        }

                        if current_difference < difference {
                            closest = (nf2 as u8, *no2);
                            difference = current_difference;
                        }
                    }
                }
            }

            ck_sys = ((hso.0 as f32 * (closest.0 as f32 / closest.1 as f32)) as u32).hz();
            ck_usb = ck_sys;

            // Map NF2 values to their respective register values
            // Refer to User manual page 88
            closest.0 = if closest.0 == 16 { 0 } else { closest.0 };

            // Map NO2 values to their respective register values
            // Refer to User manual page 88
            closest.1 = match closest.1 {
                1 => 0b00,
                2 => 0b01,
                4 => 0b10,
                8 => 0b11,
                _ => unreachable!(),
            };

            nf2 = Some(closest.0);
            no2 = Some(closest.1);
        }

        // Calculate the AHB clock prescaler
        // hclk = ck_sys / ahb prescaler
        // for the prescaler values refer to User Manual page 100
        let (ahb_div, hclk) = match self.hclk {
            Some(hclk) => {
                let (bits, div) = match ck_sys.0 / hclk.0 {
                    0 => unreachable!(),
                    1 => (0b000, 1),
                    2..=3 => (0b001, 2),
                    4..=7 => (0b010, 4),
                    8..=15 => (0b100, 8),
                    _ => (0b111, 16),
                };

                (bits, (ck_sys.0 / div).hz())
            }
            None => (0b000, ck_sys),
        };

        let stclk = (hclk.0 / 8).hz();

        // Calculate the ADC clock prescaler
        // ck_adc_ip = hclk / adc prescaler
        // for the prescaler values refer to User Manual page 103
        let (adc_div, ck_adc_ip) = match self.ck_adc_ip {
            Some(ck_adc_ip) => {
                let (bits, div) = match hclk.0 / ck_adc_ip.0 {
                    0 => unreachable!(),
                    1 => (0b000, 1),
                    2 => (0b001, 2),
                    3 => (0b111, 3),
                    4..=7 => (0b010, 4),
                    8..=15 => (0b011, 8),
                    16..=31 => (0b100, 16),
                    32..=63 => (0b101, 32),
                    _ => (0b110, 64),
                };

                (bits, (hclk.0 / div).hz())
            }
            None => (0b000, hclk),
        };

        // Apply the calculated clock configuration
        let ckcu = unsafe { &*CKCU::ptr() };

        // First configure the PLL in case it needs to be set up
        if pll_target_clock.is_some() {
            // Set the source clock for the PLL
            ckcu.ckcu_gcfgr.modify(|_, w| w.pllsrc().bit(pllsrc));

            // Set the actual configuration values
            ckcu.ckcu_pllcfgr.modify(|_, w| unsafe {
                w.pfbd() // PFBD contains NF2, refer to User Manual page 88
                    .bits(nf2.unwrap())
                    .potd() // POTD contains NO2, refer to User Manual page 88
                    .bits(no2.unwrap())
            });

            // Enable the PLL, described at User Manual page 87
            ckcu.ckcu_gccr.modify(|_, w| w.pllen().set_bit());

            // Wait for the PLL to become stable, described at User Manual page 87
            while !ckcu.ckcu_gcsr.read().pllrdy().bit_is_set() {
                cortex_m::asm::nop();
            }
        }

        // Set the flash wait states so the chip doesn't hang on higher frequencies
        // See User Manual page 66 for the values
        let fmc = unsafe { &*FMC::ptr() };
        if hclk > 24.mhz().into() {
            fmc.fmc_cfcr.modify(|_, w| unsafe { w.wait().bits(0b010) });
        }

        // Set up the proper CK_SYS source
        ckcu.ckcu_gccr.modify(|_, w| unsafe { w.sw().bits(sw) });

        // Set the AHB prescaler
        ckcu.ckcu_ahbcfgr.modify(|_, w| unsafe { w.ahbpre().bits(ahb_div) });

        // Set the ADC prescaler
        ckcu.ckcu_apbcfgr.modify(|_, w| unsafe { w.adcdiv().bits(adc_div) });

        // After all clocks are set up, configure CKOUT if required
        if let Some(ckout) = self.ckout {
            // Refer to User Manual page 94 for these values
            let ckout = match ckout {
                CkoutSrc::CkRef => 0b000,
                CkoutSrc::Hclk => 0b001,
                CkoutSrc::CkSys => 0b010,
                CkoutSrc::CkHse => 0b011,
                CkoutSrc::CkHsi => 0b100,
                CkoutSrc::CkLse => 0b101,
                CkoutSrc::CkLsi => 0b110,
            };

            ckcu.ckcu_gcfgr.modify(|_, w| unsafe { w.ckoutsrc().bits(ckout) });
        }

        Clocks {
            ckout: self.ckout,
            ck_usb,
            ck_adc_ip,
            ck_sys,
            stclk,
            hclk,
        }
    }
}
