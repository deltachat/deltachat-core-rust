ARC-Authentication-Results: i=1; mx.google.com;
       dkim=pass header.i=@mail.ru header.s=mail4 header.b=K86lQ0h9;
       spf=pass (google.com: domain of alice@mail.ru designates 94.100.181.251 as permitted sender) smtp.mailfrom=alice@mail.ru;
       dmarc=pass (p=REJECT sp=REJECT dis=NONE) header.from=mail.ru
Authentication-Results: mx.google.com;
       dkim=pass header.i=@mail.ru header.s=mail4 header.b=K86lQ0h9;
       spf=pass (google.com: domain of alice@mail.ru designates 94.100.181.251 as permitted sender) smtp.mailfrom=alice@mail.ru;
       dmarc=pass (p=REJECT sp=REJECT dis=NONE) header.from=mail.ru
From: <alice@mail.ru>
To: <alice@gmail.com>
Authentication-Results: smtpng1.m.smailru.net; auth=pass smtp.auth=alice@mail.ru smtp.mailfrom=alice@mail.ru
