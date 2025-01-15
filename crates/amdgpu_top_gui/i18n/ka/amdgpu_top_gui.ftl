info = ინფორმაცია
toggle_side_panel = გვერდითი პანელის ხილვადობის გადართვა
pause = შეჩერება
quit = გასვლა

# SidePanel
## Device Info
suspended = შეჩერებულია
device_info = მოწყობილობის ინფორმაცია
xdna_info = XDNA NPU ინფორმაცია
device_name = მოწყობილობის სახელი
pci_bus = PCI (დომენი:მატარებელი:მოწყ.ფუნქც)
did_rid = მოწყობილობისID:რევID
opengl_driver_ver = OpenGL-ის დრაივერის ვერსია
vulkan_driver_name = Vulkan-ის დრაივერის სახელი
vulkan_driver_version = Vulkan-ის დრაივერის ვრესია
rocm_ver = ROCm-ის ვერსია
gfx_target_version = gfx_target_version
apu = APU
dgpu = dGPU
rb = RenderBackend (RB)
rb_plus = RenderBackendPlus (RB+)
gp_s = GP/წმ
gflops = GFLOPS
gpu_type = GPU-ის ტიპი
family = ოჯახი
asic_name = ASIC-ის სახელი
chip_class = ჩიპის კლასი
shader_engine = Shader-ის ძრავა (SE)
shader_array_per_se = Shader-ის მასივი (SA/SH) თითოეული SE-სთვის
cu_per_sa = CU თითოეული SA-სთვის
total_cu = ჯამში CU
peak_gp = პიკური პიქსელის შევსების სიხშირე
gpu_clock = GPU-ის სიხშირე
peak_fp32 = პიკური FP32
npu = NPU
fw_version = მიკროკოდის ვერსია

enabled = ჩართულია
disabled = გამორთულია

supported = მხარდაჭერილია
not_supported = მხარდაჭერილი არაა

vram_type = VRAM-ის ტიპი
vram_bit_width = VRAM ბიტების სიგანე
vram_size = VRAM-ის ზომა
memory_clock = მეხსიერების სიხშირე
resizable_bar = ზომაცვლადი პანელი
ecc_memory = ECC მეხსიერება
ecc_memory_error_count = ECC მეხსიერების შეცდომების რაოდენობა
corrected = შესწორებულია
uncorrected = შეუსწორებელია

l1_cache_per_cu = L1 კეში (თითოეული CU-სთვის)
gl1_cache_per_sa = GL1 კეში (თითოეული SA/SH-სთვის)
l2_cache = L2 კეში
l3_cache = L3 კეში

bit = ბიტი
kib = კიბ
mib = მიბ
mib_s = მიბ/წმ
mhz = მჰც
mw = მვტ
w = ვტ
mv = მვ
ma = მა
rpm = ბრ/წთ
# Temp.
c = C
banks = მოდულები

power_cap = კვების ლიმიტი.
power_cap_default = კვების ლიმიტი. (ნაგულისხმევი)

pcie_link_speed = PCIe შეერთების სიჩქარე
pci_power_state = PCI კვების მდგომარეობა
power_profile = კვების პროფილი
# Dynamic Power Management (DPM)
dpm = DPM
max = მაქს
gpu = GPU
system = სისტემა

supported_power_profiles = მხარდაჭერილი კვების პროფილები

hw_ip_info = აპარატურული IP-ის ინფორმაცია
count = რაოდეობა
ip_type = IP-ის ტიპი
queues = რიგები

ip_discovery_table = IP-ის აღმოჩენის ცხრილი
gpu_die = მიკროსქემა
ip_hw = IP-ის აპარატურა
version = ვერსია
num = რიცხვი

video_caps_info = Video-ის ლიმიტების ინფორმაცია
codec = კოდეკი
decode = დეკოდერი
encode = ენკოდერი
n_a = N/A

vbios_info = VBIOS-ის ინფორმაცია
vbios_name = სახელი
vbios_pn = PN
vbios_version = ვერსია
vbios_date = თარიღი
vbios_size = ზომა (ბაიტი)

connector_info = კონტექტორის ინფორმაცია

# CentralPanel
## Graphics Register Bus Management (GRBM)
grbm = GRBM
grbm2 = GRBM2

## GRBM
Graphics_Pipe = გრაფიკის რიგი
Texture_Pipe = ტექსტურების რიგი
Command_Processor = ბრძანებების პროცესორი
Global_Data_Share = გლობალური მონაცემების გაზიარება
Shader_Export = Shader-ის გატანა
Shader_Processor_Interpolator = Shader-ის პროცესორის ინტერპოლატორი
Primitive_Assembly = პრიმიტივის ანაწყობი
Depth_Block = სიღრმის ბლოკი
Color_Block = ფერის ბლოკი
Geometry_Engine = გეომეტრიის ძრავა
Vertex_Grouper__Tessellator = წვეროების დამაჯგუფებელი / ტესელატორი
Input_Assembly = შეყვანის ანაწყობი
Work_Distributor = სამუშაოს განმანაწილებელი

## GRBM2
RunList_Controller = გაშვების სიის კონტროლერი
Ring_List_Controller = რგოლის სიის კონტროლერი
Texture_Cache = ტექსტურების კეში
Texture_Cache_per_Pipe = ტექსტურების კეში თითოეული რიგისთვის
Unified_Translation_Cache_Level-2 = გაერთიანებული თარგმანის კეში დონე-2
Efficiency_Arbiter = ეფექტურობის არბიტრი
Render_Backend_Memory_Interface = რენდერის უკანაბოლოს მეხსიერების ინტერფეისი
Command_Processor_-__Fetcher = ბრძანებების პროცესორი - გამომთხოვი
Command_Processor_-__Compute = ბრძანებების პროცესორი - გამომთვლელი
Command_Processor_-_Graphics = ბრძანებების პროცესორი - გრაფიკა
### System DMA
SDMA = SDMA

vram = VRAM
cpu_visible_vram = CPU-სთვის ხილული VRAM
gtt = GTT
usable = გამოყენებადი

fdinfo = fdinfo
fdinfo_plot = fdinfo-ის გრაფიკი
full_fdinfo_list = სრული სია
xdna_fdinfo = XDNA fdinfo
vram_plot = VRAM-ის გრაფიკი
cpu_temp_plot = CPU-ის ბირთვის ტემპერატურის გრაფიკი
cpu_power_plot = CPU-ის ბირთვის კვების გრაფიკი
vclk_dclk_plot = VCLK/DCLK-ის გრაფიკი
cpu = CPU
gfx = GFX
compute = გამომთვლელი
dma = DMA
# Video Core Next
vcn = VCN
# Video Compression Engine
vpe = VPE
# Process Name
name = სახელი
pid = PID

sensor = სენსორები

pcie_bw = PCIe-ის გამტარობა
sent = გაგზავნილია
received = მიღებულია

gpu_metrics = GPU-ის მეტრიკები
avg = საშ.
cur = მიმდ.
soc = SoC
memory = მეხსიერება
mc = მეხსიერების კონტროლერი
media = მედია
hbm_temp = HBM-ის ტემპ. (C)
core_temp = ბირთვის ტემპ. (C)
core_power = ბირთვის კვება (მვტ)
core_clock = ბირთვის სიხშირე (მჰც)
l3_temp = L3 კეშის ტემპ. (C)
l3_clock = L3 კეშის სიხშირე (მჰც)
socket_power = სოკეტის კვება (საშუალო)
current_socket_power = სოკეტის კვება (მიმდინარე)
avg_activity = საშუალო აქტივობა
activity = აქტივობა
throttle_status = შეზღუდვის სტატუსი
throttling_log = შეზღუდვების ჟურნალი

failed_to_set_up_gui = გრაფიკული კონტექსტის მორგება ჩავარდა.
