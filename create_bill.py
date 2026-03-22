from PIL import Image, ImageDraw, ImageFont
import textwrap
import os

def create_vietnamese_bill(merchant, amount, date, category, filename):
    # Tạo ảnh nền trắng 500x750 (tăng chiều cao ảnh một chút)
    img = Image.new('RGB', (500, 750), color=(255, 255, 255))
    draw = ImageDraw.Draw(img)
    
    # Cấu hình Font (Đảm bảo đường dẫn font Arial đúng trên máy của bạn)
    try:
        font_bold = ImageFont.truetype("arialbd.ttf", 24)
        font_reg = ImageFont.truetype("arial.ttf", 18)
    except:
        font_bold = ImageFont.load_default()
        font_reg = ImageFont.load_default()

    # Tiêu đề hóa đơn
    draw.text((120, 40), "HÓA ĐƠN TIỀN ĐIỆN (GTGT)", fill=(0, 0, 0), font=font_bold)
    
    # Phần Đơn vị bán hàng (Xử lý xuống dòng)
    merchant_label = "Đơn vị bán hàng: "
    draw.text((50, 100), merchant_label, fill=(0, 0, 0), font=font_reg)
    
    wrapped_merchant = textwrap.fill(merchant, width=22)
    
    draw.multiline_text((50 + draw.textlength(merchant_label, font=font_reg), 100), wrapped_merchant, fill=(0, 0, 0), font=font_reg, spacing=5)
    
    # Tính toán tọa độ Y mới dựa trên số dòng
    num_lines = wrapped_merchant.count('\n') + 1
    new_y = 100 + (num_lines * 25) 
    
    # Các thông tin khác
    draw.text((50, new_y), f"Mã khách hàng: KH99887722", fill=(0, 0, 0), font=font_reg)
    draw.text((50, new_y + 40), f"Ngày lập hóa đơn: {date}", fill=(0, 0, 0), font=font_reg)
    draw.text((50, new_y + 80), f"Phân loại: {category}", fill=(0, 0, 0), font=font_reg)
    
    # Đường kẻ ngang thứ 1
    current_y = new_y + 120
    draw.line((50, current_y, 450, current_y), fill=(0, 0, 0), width=2)
    
    # Chi tiết tiền
    current_y += 40
    total_label = "Tổng cộng tiền hàng: "
    total_val = f"{amount:,.0f} VNĐ"
    
    draw.text((50, current_y), total_label, fill=(0, 0, 0), font=font_reg)
    # Căn phải số tiền
    draw.text((450 - draw.textlength(total_val, font=font_bold), current_y), total_val, fill=(0, 0, 0), font=font_bold)
    
    # Thuế GTGT
    current_y += 40
    tax_label = "Thuế suất GTGT (10%): "
    tax_val = f"{amount*0.1:,.0f} VNĐ"
    draw.text((50, current_y), tax_label, fill=(0, 0, 0), font=font_reg)
    draw.text((450 - draw.textlength(tax_val, font=font_reg), current_y), tax_val, fill=(0, 0, 0), font=font_reg)
    
    # Đường kẻ ngang thứ 2
    current_y += 60
    draw.line((50, current_y, 450, current_y), fill=(0, 0, 0), width=2)
    
    # blind spot: Đẩy "TỔNG TIỀN THANH TOÁN" xuống thật xa
    # Đổi tọa độ Y thành 520
    pay_label_y = current_y + 80
    pay_val_y = current_y + 130 # Đẩy số tiền đỏ xuống thấp hơn chữ "TỔNG TIỀN"
    
    draw.text((50, pay_label_y), "TỔNG TIỀN THANH TOÁN:", fill=(255, 0, 0), font=font_bold)
    
    # Dùng Multiline Text cho số tiền đỏ để đảm bảo nó nằm đẹp dưới chữ "TỔNG TIỀN"
    pay_val = f"{(amount*1.1):,.0f} VNĐ"
    draw.multiline_text((450 - draw.textlength(pay_val, font=font_bold), pay_val_y), pay_val, fill=(255, 0, 0), font=font_bold, align="right")
    
    # Lời cảm ơn
    draw.text((100, 690), "Cảm ơn Quý khách đã sử dụng dịch vụ!", fill=(100, 100, 100), font=font_reg)

    os.makedirs("./bill", exist_ok=True)
    img.save("./bill/" + filename)
    print(f"Đã tạo hóa đơn: {filename}")

# # Tạo mẫu hóa đơn Điện để test
create_vietnamese_bill("TỔNG CÔNG TY ĐIỆN LỰC MIỀN BẮC - CHI NHÁNH EVN HÀ NỘI", 534796, "2026-01-20", "Điện", "bill_dien_1.jpg")
create_vietnamese_bill("TỔNG CÔNG TY ĐIỆN LỰC MIỀN BẮC - CHI NHÁNH EVN HÀ NỘI", 514856, "2026-02-20", "Điện", "bill_dien_2.jpg")
create_vietnamese_bill("TỔNG CÔNG TY ĐIỆN LỰC MIỀN BẮC - CHI NHÁNH EVN HÀ NỘI", 509872, "2026-03-20", "Điện", "bill_dien_3.jpg")
# Tạo mẫu hóa đơn Nước để test
create_vietnamese_bill("TỔNG CÔNG TY NƯỚC SẠCH HÀ NỘI", 205716, "2026-01-20", "Nước", "bill_nuoc_1.jpg")
create_vietnamese_bill("TỔNG CÔNG TY NƯỚC SẠCH HÀ NỘI", 195896, "2026-02-20", "Nước", "bill_nuoc_2.jpg")
create_vietnamese_bill("TỔNG CÔNG TY NƯỚC SẠCH HÀ NỘI", 210561, "2026-03-20", "Nước", "bill_nuoc_3.jpg")