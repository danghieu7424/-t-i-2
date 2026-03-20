use lettre::{Message, SmtpTransport, Transport};
use lettre::transport::smtp::authentication::Credentials;
use lettre::message::header::ContentType; // <-- Thêm cái này
use std::env;

// Hàm gửi mail chung (đã nâng cấp lên HTML)
fn send_email(to_email: String, subject: String, body_html: String) {
    let gmail_user = env::var("GMAIL_USER").expect("Thiếu GMAIL_USER");
    let gmail_pass = env::var("GMAIL_PASS").expect("Thiếu GMAIL_PASS");

    let email = Message::builder()
        .from(gmail_user.parse().unwrap())
        .to(to_email.parse().unwrap())
        .subject(subject)
        .header(ContentType::TEXT_HTML) // <-- QUAN TRỌNG: Báo cho Gmail biết đây là HTML
        .body(body_html)
        .unwrap();

    let creds = Credentials::new(gmail_user, gmail_pass);
    let mailer = SmtpTransport::relay("smtp.gmail.com")
        .unwrap()
        .credentials(creds)
        .build();

    std::thread::spawn(move || {
        match mailer.send(&email) {
            Ok(_) => println!("Email sent to {}", to_email),
            Err(e) => eprintln!("Error sending email: {:?}", e),
        }
    });
}

// Hàm tạo nội dung HTML cho đơn hàng đang giao
pub fn send_order_shipping_email(to_email: String, order_id: String, items_rows_html: String, total_amount: String) {
    let subject = format!("📦 Đơn hàng #{} đang được vận chuyển!", order_id);
    
    // Dùng HTML để vẽ bảng
    let body = format!(r#"
        <!DOCTYPE html>
        <html>
        <head>
            <style>
                body {{ font-family: Arial, sans-serif; line-height: 1.6; color: #333; }}
                .container {{ max-width: 600px; margin: 0 auto; padding: 20px; border: 1px solid #ddd; border-radius: 10px; }}
                h2 {{ color: #2563EB; }}
                table {{ width: 100%; border-collapse: collapse; margin-top: 20px; }}
                th, td {{ padding: 12px; border-bottom: 1px solid #ddd; text-align: left; }}
                th {{ background-color: #f8f9fa; }}
                .total {{ text-align: right; font-size: 18px; font-weight: bold; color: #d9534f; margin-top: 20px; }}
                .footer {{ margin-top: 30px; font-size: 12px; color: #777; text-align: center; }}
            </style>
        </head>
        <body>
            <div class="container">
                <h2>Đơn hàng đang trên đường đến bạn!</h2>
                <p>Xin chào,</p>
                <p>Đơn hàng <b>#{}</b> của bạn đã được đóng gói và bàn giao cho đơn vị vận chuyển.</p>
                
                <h3>Chi tiết đơn hàng:</h3>
                <table>
                    <thead>
                        <tr>
                            <th>Sản phẩm</th>
                            <th style="text-align: center;">SL</th>
                            <th style="text-align: right;">Đơn giá</th>
                            <th style="text-align: right;">Thành tiền</th>
                        </tr>
                    </thead>
                    <tbody>
                        {} 
                    </tbody>
                </table>

                <div class="total">
                    Tổng thanh toán: {}
                </div>

                <p>Vui lòng chú ý điện thoại để nhận hàng nhé!</p>
                
                <div class="footer">
                    Trân trọng,<br>
                    <b>ElectroShop Team</b>
                </div>
            </div>
        </body>
        </html>
    "#, order_id, items_rows_html, total_amount);

    send_email(to_email, subject, body);
}

// Hàm cảm ơn (cũng chuyển sang HTML cho đẹp)
pub fn send_order_thank_you_email(to_email: String, order_id: String, points: i32) {
    let subject = format!("✅ Cảm ơn bạn đã mua sắm (Đơn #{})", order_id);
    let body = format!(r#"
        <div style="font-family: Arial; padding: 20px; border: 1px solid #eee; border-radius: 8px;">
            <h2 style="color: #16a34a;">Giao hàng thành công!</h2>
            <p>Xin chào,</p>
            <p>Cảm ơn bạn đã xác nhận nhận hàng thành công đơn <b>#{}</b>.</p>
            <p style="background-color: #ecfdf5; color: #065f46; padding: 15px; border-radius: 5px; text-align: center; font-weight: bold;">
                🎉 Bạn đã được cộng +{} điểm thưởng vào ví.
            </p>
            <p>Hy vọng bạn hài lòng với sản phẩm. Đừng quên để lại đánh giá nhé!</p>
            <br>
            <p>ElectroShop Team</p>
        </div>
    "#, order_id, points);

    send_email(to_email, subject, body);
}