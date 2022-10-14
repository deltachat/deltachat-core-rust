ARC-Authentication-Results: i=1; mx.google.com;
       dkim=pass header.i=@icloud.com header.s=1a1hai header.b=l1YS4V6g;
       spf=pass (google.com: domain of alice@icloud.com designates 17.57.155.16 as permitted sender) smtp.mailfrom=alice@icloud.com;
       dmarc=pass (p=QUARANTINE sp=QUARANTINE dis=NONE) header.from=icloud.com
Authentication-Results: mx.google.com;
       dkim=pass header.i=@icloud.com header.s=1a1hai header.b=l1YS4V6g;
       spf=pass (google.com: domain of alice@icloud.com designates 17.57.155.16 as permitted sender) smtp.mailfrom=alice@icloud.com;
       dmarc=pass (p=QUARANTINE sp=QUARANTINE dis=NONE) header.from=icloud.com
From: <alice@icloud.com>
To: <alice@gmail.com>
