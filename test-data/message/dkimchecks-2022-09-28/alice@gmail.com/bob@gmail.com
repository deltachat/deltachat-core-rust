ARC-Authentication-Results: i=1; mx.google.com;
       dkim=pass header.i=@gmail.com header.s=20210112 header.b=UpITNcvK;
       spf=pass (google.com: domain of bob@gmail.com designates 209.85.220.41 as permitted sender) smtp.mailfrom=bob@gmail.com;
       dmarc=pass (p=NONE sp=QUARANTINE dis=NONE) header.from=gmail.com
Authentication-Results: mx.google.com;
       dkim=pass header.i=@gmail.com header.s=20210112 header.b=UpITNcvK;
       spf=pass (google.com: domain of bob@gmail.com designates 209.85.220.41 as permitted sender) smtp.mailfrom=bob@gmail.com;
       dmarc=pass (p=NONE sp=QUARANTINE dis=NONE) header.from=gmail.com
From: Bob <bob@gmail.com>
To: "alice@gmail.com" <alice@gmail.com>
